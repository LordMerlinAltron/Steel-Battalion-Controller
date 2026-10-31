# Architecture

```
SBC ──USB (WinUSB)──► sbc-usb ──[u8;26]──► sbc-core::decode ──SbcState──► sbc-core::map ──PadState──► sbc-vjoy ──► vJoy ──► JoyToKey
                         ▲                         │
                         │                         └──► sbc-core::led (latch) ──► LED encode ──┐
                         └──────────────────────────────────── LED OUT report ◄────────────────┘
```

## Crates

| Crate | Kind | I/O | `unsafe` | Job |
|---|---|---|---|---|
| `sbc-core` | lib | none | forbidden | decode input, map to pad state, LED latch, LED encode |
| `sbc-usb` | lib | USB | forbidden | find/open SBC, read IN, write OUT (via `nusb`) |
| `sbc-vjoy` | lib | vJoy DLL | FFI only | acquire device, set axes/buttons (P3) |
| `sbc-capture` | bin | USB | forbidden | evidence tool: descriptors, live controls, LED probe |
| `sbc-bridge` | bin | USB + vJoy | forbidden | runtime: hotplug loop, wiring, config (P3) |

## Rules

- `sbc-core` has no dependencies on USB, vJoy, OS, or config files. Testable from byte fixtures.
- Adapters own all I/O. Bins only wire adapters to core.
- Fixed-size state (`u64` masks, fixed arrays). No allocation in the read loop.
- Every loop bounded (iteration cap or timeout).
- Zero warnings: `clippy -D warnings` (pedantic) + `cargo-deny`.

## Why not a kernel driver

The 2019 driver (caosdoar/SteelBattalionDriver, GPLv3) is unsigned. Windows x64 needs test-signing mode and Secure Boot off to load it.
WinUSB is inbox and Microsoft-signed; vJoy ships signed builds. This repo contains no code from the 2019 driver or SteelBattalion.NET; the protocol facts are documented in [protocol.md](protocol.md).

Prior art: Steel Battalion 64 / SteelBattalion.NET (C#, LibUsbDotNet + vJoy) used this same user-mode pattern. This project rebuilds it with a compiled bridge, no script runtime, and JoyToKey as the binding layer.
