# Steel Battalion Controller for Windows

Use the original Xbox **Steel Battalion** controller on Windows 10/11 as a joystick, mapped to keyboard and mouse with **JoyToKey**. Button LEDs light up and toggle on press.

No custom kernel driver. No test-signing mode. Secure Boot stays on.

```
SBC ──WinUSB──► sbc-bridge ──► vJoy ──► JoyToKey ──► keys / mouse
```

> Status: early development. See [ROADMAP.md](ROADMAP.md).

## Docs
- [Roadmap](ROADMAP.md)
- [Architecture](docs/architecture.md)
- [USB protocol](docs/protocol.md)
- [Control map](docs/control-map.md) · [Layout reference](docs/controller-layout.png)

## Requirements
- Windows 10/11 x64
- [vJoy](https://github.com/jshafer817/vJoy) (signed build)
- [Zadig](https://zadig.akeo.ie/) — one-time WinUSB binding for the controller
- [JoyToKey](https://joytokey.net/) 6.2+ (128-button support)

## Build
```powershell
cargo build --release --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Capture tool (Phase 1)
```powershell
cargo run -p sbc-capture -- info        # USB descriptors + endpoints
cargo run -p sbc-capture -- watch       # live named controls
cargo run -p sbc-capture -- raw         # raw 26-byte packets as hex
cargo run -p sbc-capture -- led-probe   # light each LED id, print expected control
```

## Credits
Protocol knowledge from:
- [xboxdevwiki — Xbox Input Devices](https://xboxdevwiki.net/Xbox_Input_Devices) (USB/XID descriptors, 22-byte LED report, tuner and gear semantics)
- [caosdoar/SteelBattalionDriver](https://github.com/caosdoar/SteelBattalionDriver) — Oscar Sebio Cajaraville, 2019 (input layout)
- [SteelBattalion.NET](https://github.com/dmanning23/SteelBattalion.NET) — Joseph Coutcher, 2010 (endpoints, LED ids)

No code from these projects is included. See [docs/protocol.md](docs/protocol.md) for which source backs which fact.

## License
MIT — see [LICENSE](LICENSE).
