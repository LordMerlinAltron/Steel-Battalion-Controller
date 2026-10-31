//! Xbox XID descriptor (vendor `GET_DESCRIPTOR`, type `0x42`). Pure parsing.
//!
//! Request: `bmRequestType 0xC1, bRequest 6, wValue 0x4200, wIndex <interface>, wLength 16`.
//! Source: xboxdevwiki "Xbox Input Devices".

/// `bRequest` for the XID descriptor.
pub const XID_REQUEST: u8 = 6;
/// `wValue` for the XID descriptor.
pub const XID_VALUE: u16 = 0x4200;
/// Descriptor length.
pub const XID_LEN: usize = 16;
/// `bDescriptorType`.
pub const XID_DESCRIPTOR_TYPE: u8 = 0x42;
/// `bType` of the Steel Battalion controller.
pub const XID_TYPE_STEEL_BATTALION: u8 = 0x80;

/// Parsed XID descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XidDescriptor {
    /// `bcdXid`.
    pub bcd_xid: u16,
    /// `bType` (0x80 = Steel Battalion).
    pub device_type: u8,
    /// `bSubType`.
    pub sub_type: u8,
    /// `bMaxInputReportSize`.
    pub max_input: u8,
    /// `bMaxOutputReportSize`.
    pub max_output: u8,
}

/// Why a descriptor was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XidError {
    /// Fewer than 8 bytes.
    TooShort(usize),
    /// `bDescriptorType` is not 0x42.
    WrongType(u8),
    /// `bLength` disagrees with the data received.
    BadLength(u8),
}

/// Parse the descriptor bytes returned by the device.
///
/// # Errors
/// Short data, wrong descriptor type, or inconsistent length.
pub fn parse(bytes: &[u8]) -> Result<XidDescriptor, XidError> {
    let Some(
        &[
            len,
            ty,
            b0,
            b1,
            device_type,
            sub_type,
            max_input,
            max_output,
        ],
    ) = bytes.first_chunk::<8>()
    else {
        return Err(XidError::TooShort(bytes.len()));
    };
    if ty != XID_DESCRIPTOR_TYPE {
        return Err(XidError::WrongType(ty));
    }
    if usize::from(len) < 8 || usize::from(len) > bytes.len() {
        return Err(XidError::BadLength(len));
    }
    let d = XidDescriptor {
        bcd_xid: u16::from_le_bytes([b0, b1]),
        device_type,
        sub_type,
        max_input,
        max_output,
    };
    debug_assert!(bytes.len() >= 8);
    debug_assert_eq!(d.device_type, device_type);
    Ok(d)
}

impl XidDescriptor {
    /// True for the Steel Battalion controller.
    #[must_use]
    pub const fn is_steel_battalion(&self) -> bool {
        self.device_type == XID_TYPE_STEEL_BATTALION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{INPUT_REPORT_LEN, LED_REPORT_LEN};

    /// Example from xboxdevwiki for this controller.
    const WIKI: [u8; 16] = [
        0x10, 0x42, 0x00, 0x01, 0x80, 0x01, 0x1A, 0x16, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF,
    ];

    #[test]
    fn parses_wiki_example_and_matches_our_report_sizes() {
        let d = parse(&WIKI).unwrap();
        assert!(d.is_steel_battalion());
        assert_eq!(d.bcd_xid, 0x0100);
        assert_eq!(usize::from(d.max_input), INPUT_REPORT_LEN);
        assert_eq!(usize::from(d.max_output), LED_REPORT_LEN);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse(&WIKI[..4]), Err(XidError::TooShort(4)));
        let mut wrong = WIKI;
        wrong[1] = 0x21;
        assert_eq!(parse(&wrong), Err(XidError::WrongType(0x21)));
        assert_eq!(parse(&WIKI[..10]), Err(XidError::BadLength(0x10)));
    }
}
