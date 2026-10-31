//! USB adapter for the Steel Battalion controller. Owns all USB I/O.
//!
//! Windows: the controller must be bound to `WinUSB` (Zadig) once.
//! Buffers are allocated at open and reused; the read path does not allocate.

#![forbid(unsafe_code)]

use std::fmt;
use std::time::Duration;

use nusb::descriptors::TransferType;
use nusb::transfer::{
    Buffer, ControlIn, ControlType, Direction, In, Interrupt, Out, Recipient, TransferError,
};
use nusb::{Device, DeviceInfo, Endpoint, Interface, MaybeFuture};

/// USB vendor id.
pub const VID: u16 = 0x0A7B;
/// USB product id.
pub const PID: u16 = 0xD000;
/// Interface claimed for input/LED traffic.
pub const INTERFACE: u8 = 0;
/// Largest report accepted by [`SbcDevice::read`] / [`SbcDevice::write`].
pub const MAX_REPORT: usize = 64;
/// IN transfers kept in flight.
const IN_FLIGHT: usize = 2;
/// Timeout for control requests.
const CONTROL_TIMEOUT: Duration = Duration::from_millis(500);

/// Adapter errors.
#[derive(Debug)]
pub enum UsbError {
    /// No controller with [`VID`]/[`PID`] present.
    NotFound,
    /// Enumeration / open / claim failed (often: `WinUSB` not bound).
    Open(nusb::Error),
    /// Interface 0 has no interrupt IN endpoint.
    NoInEndpoint,
    /// Interface 0 has no interrupt OUT endpoint.
    NoOutEndpoint,
    /// Transfer failed.
    Transfer(TransferError),
    /// Device unplugged.
    Disconnected,
    /// Caller buffer or report size invalid.
    BadLength(usize),
}

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "controller {VID:04X}:{PID:04X} not found"),
            Self::Open(e) => write!(f, "open failed: {e} (is WinUSB bound via Zadig?)"),
            Self::NoInEndpoint => write!(f, "no interrupt IN endpoint on interface {INTERFACE}"),
            Self::NoOutEndpoint => write!(f, "no interrupt OUT endpoint on interface {INTERFACE}"),
            Self::Transfer(e) => write!(f, "transfer failed: {e:?}"),
            Self::Disconnected => write!(f, "controller disconnected"),
            Self::BadLength(n) => write!(f, "invalid length {n} (max {MAX_REPORT})"),
        }
    }
}

impl std::error::Error for UsbError {}

impl From<TransferError> for UsbError {
    fn from(e: TransferError) -> Self {
        match e {
            TransferError::Disconnected => Self::Disconnected,
            other => Self::Transfer(other),
        }
    }
}

/// Find the controller in the current device list.
///
/// # Errors
/// [`UsbError::Open`] if enumeration fails, [`UsbError::NotFound`] if absent.
pub fn find() -> Result<DeviceInfo, UsbError> {
    let mut devices = nusb::list_devices().wait().map_err(UsbError::Open)?;
    devices
        .find(|d| d.vendor_id() == VID && d.product_id() == PID)
        .ok_or(UsbError::NotFound)
}

/// Interrupt endpoint addresses found on [`INTERFACE`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndpointAddrs {
    /// Interrupt IN address (bit 7 set).
    pub input: Option<u8>,
    /// Interrupt OUT address.
    pub output: Option<u8>,
}

/// Scan the active configuration for interrupt endpoints on [`INTERFACE`].
#[must_use]
pub fn endpoint_addrs(device: &Device) -> EndpointAddrs {
    let mut found = EndpointAddrs::default();
    let Ok(config) = device.active_configuration() else {
        return found;
    };
    let alts = config
        .interfaces()
        .filter(|i| i.interface_number() == INTERFACE)
        .flat_map(|i| i.alt_settings().take(1).collect::<Vec<_>>());
    for alt in alts {
        for ep in alt
            .endpoints()
            .filter(|e| e.transfer_type() == TransferType::Interrupt)
        {
            match ep.direction() {
                Direction::In if found.input.is_none() => found.input = Some(ep.address()),
                Direction::Out if found.output.is_none() => found.output = Some(ep.address()),
                _ => {}
            }
        }
    }
    found
}

/// Open controller session.
pub struct SbcDevice {
    device: Device,
    interface: Interface,
    ep_in: Endpoint<Interrupt, In>,
    ep_out: Option<Endpoint<Interrupt, Out>>,
    out_buf: Option<Buffer>,
    addrs: EndpointAddrs,
}

impl SbcDevice {
    /// Find, open, claim interface 0, and start IN transfers.
    ///
    /// # Errors
    /// Not found, open/claim failure, or missing IN endpoint. Missing OUT is not an error.
    pub fn open() -> Result<Self, UsbError> {
        let info = find()?;
        let device = info.open().wait().map_err(UsbError::Open)?;
        let interface = device
            .claim_interface(INTERFACE)
            .wait()
            .map_err(UsbError::Open)?;
        let addrs = endpoint_addrs(&device);

        let in_addr = addrs.input.ok_or(UsbError::NoInEndpoint)?;
        let mut ep_in = interface
            .endpoint::<Interrupt, In>(in_addr)
            .map_err(UsbError::Open)?;
        let packet = ep_in.max_packet_size();
        if packet == 0 || packet > MAX_REPORT {
            return Err(UsbError::BadLength(packet));
        }
        for _ in 0..IN_FLIGHT {
            let buf = ep_in.allocate(packet);
            ep_in.submit(buf);
        }

        let ep_out = match addrs.output {
            Some(a) => Some(
                interface
                    .endpoint::<Interrupt, Out>(a)
                    .map_err(UsbError::Open)?,
            ),
            None => None,
        };
        let out_buf = ep_out.as_ref().map(|_| Buffer::new(MAX_REPORT));
        debug_assert_eq!(ep_in.pending(), IN_FLIGHT);
        debug_assert_eq!(ep_out.is_some(), out_buf.is_some());

        Ok(Self {
            device,
            interface,
            ep_in,
            ep_out,
            out_buf,
            addrs,
        })
    }

    /// Endpoint addresses in use.
    #[must_use]
    pub const fn addrs(&self) -> EndpointAddrs {
        self.addrs
    }

    /// Read the raw XID descriptor (vendor request, see `sbc_core::xid`). Not on the hot path.
    ///
    /// # Errors
    /// Transfer failure (device may stall if unsupported).
    pub fn xid_descriptor(&self) -> Result<Vec<u8>, UsbError> {
        let req = ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: sbc_core::xid::XID_REQUEST,
            value: sbc_core::xid::XID_VALUE,
            index: u16::from(INTERFACE),
            length: u16::try_from(sbc_core::xid::XID_LEN).unwrap_or(16),
        };
        let bytes = self
            .interface
            .control_in(req, CONTROL_TIMEOUT)
            .wait()
            .map_err(UsbError::from)?;
        debug_assert!(bytes.len() <= sbc_core::xid::XID_LEN);
        debug_assert_eq!(self.interface.interface_number(), INTERFACE);
        Ok(bytes)
    }

    /// OUT endpoint max packet size, if an OUT endpoint exists.
    #[must_use]
    pub fn out_max_packet(&self) -> Option<usize> {
        self.ep_out.as_ref().map(Endpoint::max_packet_size)
    }

    /// Underlying device (descriptor dumps).
    #[must_use]
    pub const fn device(&self) -> &Device {
        &self.device
    }

    /// Wait up to `timeout` for one input report; copy it into `out`.
    /// Returns bytes copied, or `Ok(None)` on timeout.
    ///
    /// # Errors
    /// Disconnect or transfer failure. The transfer is resubmitted before returning
    /// unless the device is gone.
    pub fn read(
        &mut self,
        out: &mut [u8; MAX_REPORT],
        timeout: Duration,
    ) -> Result<Option<usize>, UsbError> {
        let Some(done) = self.ep_in.wait_next_complete(timeout) else {
            return Ok(None);
        };
        let status = done.status;
        let n = done.actual_len.min(MAX_REPORT);
        if let (Some(dst), Some(src)) = (out.get_mut(..n), done.buffer.get(..n)) {
            dst.copy_from_slice(src);
        }
        if status != Err(TransferError::Disconnected) {
            let mut buf = done.buffer;
            buf.set_requested_len(self.ep_in.max_packet_size());
            self.ep_in.submit(buf);
        }
        status?;
        debug_assert!(n <= MAX_REPORT);
        debug_assert!(self.ep_in.pending() > 0);
        Ok(Some(n))
    }

    /// Send one OUT report (LEDs). Blocks up to `timeout`.
    ///
    /// # Errors
    /// No OUT endpoint, bad length, or transfer failure.
    pub fn write(&mut self, data: &[u8], timeout: Duration) -> Result<(), UsbError> {
        if data.is_empty() || data.len() > MAX_REPORT {
            return Err(UsbError::BadLength(data.len()));
        }
        let (Some(ep), Some(mut buf)) = (self.ep_out.as_mut(), self.out_buf.take()) else {
            return Err(UsbError::NoOutEndpoint);
        };
        buf.clear();
        buf.extend_from_slice(data);
        let done = ep.transfer_blocking(buf, timeout);
        let status = done.status;
        self.out_buf = Some(done.buffer);
        debug_assert!(self.out_buf.is_some());
        status.map_err(UsbError::from)
    }
}
