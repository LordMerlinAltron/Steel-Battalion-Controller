# Roadmap

Goal: plug in the Steel Battalion controller → Windows sees a joystick → JoyToKey profile maps it to keyboard/mouse. LEDs work (toggle on press).

Approach: user-mode bridge. **No custom kernel driver, no driver signing, Secure Boot stays on.**

## P0 — Prereqs and risk check
- [ ] Remove 2019 test-signed driver if installed; turn test-signing off
- [ ] Install signed vJoy; confirm `vJoy Device` status OK with Secure Boot ON  ← **blocker if this fails**
- [ ] Bind SBC to WinUSB with Zadig
- [ ] Rust toolchain + MSVC build tools
- [ ] Repo scaffold (this branch)

## P1 — Evidence capture (`sbc-capture`)
- [ ] `info`: confirm endpoints IN `0x82` / OUT `0x01` → `docs/protocol.md`
- [ ] `watch`: press every control, confirm name/bit → `docs/control-map.md` (🟡 → ✅)
- [ ] Record gear raw values; confirm trigger mask (`0x02` vs `0x03`)
- [ ] `raw --save`: save packet fixtures for tests (`fixtures/*.bin`)
- [ ] `led-probe`: confirm 34-byte nibble map, LED ids 4–33 + gears 35–41
- [ ] Check for switch bounce (decides debounce)

## P2 — Core logic (`sbc-core`, pure)
- [x] Input decode (26 bytes)
- [x] LED latch (toggle / mirror)
- [x] Pad mapping (axes, gear/tuner buttons, toggle pulses)
- [x] LED encode (34-byte report, gear position/progressive)
- [ ] Tests from real fixtures

## P3 — Bridge (`sbc-bridge`)
- [ ] vJoy FFI adapter (`sbc-vjoy`, only crate with `unsafe`)
- [ ] Hotplug watch → open → bounded read loop with timeout
- [ ] Disconnect → vJoy neutral (no stuck keys)
- [ ] LED writes on change only
- [ ] Config from `config/default.toml`

## P4 — Plug and play
- [ ] Start at logon (Task Scheduler), hidden, single instance
- [ ] Log file, rotate
- [ ] Optional tray icon

## P5 — JoyToKey
- [ ] Configure vJoy device: 8 axes, 80 buttons
- [ ] First profile → `joytokey/`
- [ ] Setup guide in README

## P6 — Release
- [ ] CI green on Windows (fmt, clippy `-D warnings`, test, cargo-deny)
- [ ] Release zip: `sbc-bridge.exe`, `sbc-capture.exe`, config, install script
