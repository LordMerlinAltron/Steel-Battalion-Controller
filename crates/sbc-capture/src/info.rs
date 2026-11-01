//! `info`: dump descriptors so endpoint addresses can be recorded in docs/protocol.md.

use nusb::MaybeFuture;
use sbc_usb::UsbError;

pub fn run() -> Result<(), UsbError> {
    let info = sbc_usb::find()?;
    println!(
        "device   {:04X}:{:04X}",
        info.vendor_id(),
        info.product_id()
    );
    println!("product  {}", info.product_string().unwrap_or("-"));
    println!("maker    {}", info.manufacturer_string().unwrap_or("-"));
    println!(
        "class    {:02X}/{:02X}/{:02X}",
        info.class(),
        info.subclass(),
        info.protocol()
    );

    let device = info.open().wait().map_err(UsbError::Open)?;
    let mut configs = 0usize;
    for config in device.configurations().take(8) {
        configs = configs.saturating_add(1);
        println!("config {}", config.configuration_value());
        for iface in config.interfaces().take(16) {
            for alt in iface.alt_settings().take(8) {
                println!(
                    "  interface {} alt {} class {:02X}/{:02X}/{:02X} endpoints {}",
                    alt.interface_number(),
                    alt.alternate_setting(),
                    alt.class(),
                    alt.subclass(),
                    alt.protocol(),
                    alt.num_endpoints()
                );
                for ep in alt.endpoints().take(16) {
                    println!(
                        "    ep 0x{:02X} {:?} {:?} max_packet {} interval {}",
                        ep.address(),
                        ep.direction(),
                        ep.transfer_type(),
                        ep.max_packet_size(),
                        ep.interval()
                    );
                }
            }
        }
    }
    let addrs = sbc_usb::endpoint_addrs(&device);
    println!("selected IN {:02X?} OUT {:02X?}", addrs.input, addrs.output);
    debug_assert!(configs <= 8);
    if configs == 0 {
        println!("warning: no configuration descriptors returned");
    }
    drop(device);
    print_xid();
    Ok(())
}

/// XID descriptor: the device's own statement of its report sizes.
fn print_xid() {
    let dev = match sbc_usb::SbcDevice::open() {
        Ok(d) => d,
        Err(e) => return println!("XID: cannot open for control request: {e}"),
    };
    match dev.xid_descriptor() {
        Ok(raw) => {
            println!("XID raw {raw:02X?}");
            match sbc_core::xid::parse(&raw) {
                Ok(x) => println!(
                    "XID bcd {:04X} type {:02X} sub {:02X} max_in {} max_out {} (expect 80 01 26 22)",
                    x.bcd_xid, x.device_type, x.sub_type, x.max_input, x.max_output
                ),
                Err(e) => println!("XID parse error: {e:?}"),
            }
        }
        Err(e) => println!("XID request failed: {e}"),
    }
}
