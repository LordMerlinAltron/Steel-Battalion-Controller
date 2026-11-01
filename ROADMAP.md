# Roadmap

Goal: plug in the Steel Battalion controller → Windows sees a joystick → JoyToKey profile maps it to keyboard/mouse. LEDs work (toggle on press).

Approach: user-mode bridge. **No custom kernel driver, no driver signing, Secure Boot stays on.**

## P0 — Prereqs and risk check
- [ ] Remove 2019 test-signed driver if installed; turn test-signing off
- [ ] Install signed vJoy; confirm `vJoy Device` status OK with Secure Boot ON  ← **blocker if this fails**
- [ ] Bind SBC to WinUSB with Zadig
- [ ] Rust toolchain + MSVC build tools
- [x] Repo scaffold (this branch)
- [ ] Run `scripts/p0-check.ps1` (admin) → `logs/p0-check.txt`

## P1 — Evidence capture (`sbc-capture`) — run `scripts/p1-capture.ps1`
- [ ] `info`: confirm endpoints IN `0x82` / OUT `0x01` and XID sizes 26 / 22 → `docs/protocol.md`
- [ ] `watch`: press every control, confirm name/bit → `docs/control-map.md` (🟡 → ✅)
- [ ] Record gear raw values; confirm trigger mask (`0x02` vs `0x03`)
- [ ] `raw --save`: save packet fixtures for tests (`fixtures/*.bin`)
- [ ] `led-probe`: confirm 22-byte report (header `00 16`), LED ids 4–33 + gears 35–41
- [ ] Check for switch bounce (decides debounce)

## P2 — Core logic (`sbc-core`, pure)
- [x] Input decode (26 bytes)
- [x] LED latch (toggle / mirror)
- [x] Pad mapping (axes, gear/tuner buttons, toggle pulses)
- [x] LED encode (22-byte report with header, gear position/progressive, hold gear through neutral gap)
- [x] XID descriptor parse; bridge checks output size at connect
- [ ] Tests from real fixtures

## P3 — Bridge (`sbc-bridge`)
- [x] vJoy FFI adapter (`sbc-vjoy`, only crate with `unsafe`) — Windows build unverified
- [x] Reconnect loop → open → bounded read loop with timeout
- [x] Disconnect → vJoy neutral (no stuck keys)
- [x] LED writes on change only; hold Start 2 s = all LEDs off
- [x] Config from `sbc-bridge.toml` next to exe, else `config/default.toml`

## P4 — Plug and play
- [x] Start at logon (Startup shortcut, no admin), hidden (no console in release builds), single instance (exclusive lock file)
- [x] Log file `logs\bridge.log`, rotated at 1 MB
- [x] Clean stop via `bin\sbc-bridge.stop` (vJoy → neutral); waits for vJoy at boot
- [x] `setup\5-install-bridge.cmd`, `bridge-start/stop.cmd`, `6-uninstall-bridge.cmd`
- [ ] Verified on hardware
- [ ] Optional tray icon

## P5 — JoyToKey
- [ ] Configure vJoy device: 8 axes, 80 buttons
- [ ] First profile → `joytokey/`
- [ ] Setup guide in README

## P6 — Release
- [ ] CI green on Windows (fmt, clippy `-D warnings`, test, cargo-deny)
- [ ] Release zip: `sbc-bridge.exe`, `sbc-capture.exe`, config, install script
