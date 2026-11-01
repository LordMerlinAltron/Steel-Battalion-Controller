# Setup (double-click in order)

Run from the project folder (`setup\...`). Every step writes a log to `logs\`.

| Step | File / action | Admin | What it does |
|---|---|---|---|
| 1 | `1-check.cmd` | yes (UAC) | Read-only check → `logs\p0-check.txt` |
| 2 | `2-install-rust.cmd` | UAC once | Rust MSVC toolchain (same as CI) + VS 2022 C++ Build Tools (~4-6 GB) → `logs\step2-rust.txt` |
| 3a | Install **vJoy 2.1.9.1** (once) | yes | https://github.com/jshafer817/vJoy/releases/tag/v2.1.9.1 → `vJoySetup.exe`, reboot |
| 3b | `3-drivers.cmd` | yes (UAC) | Configures vJoy device 1 (8 axes, 80 buttons) via `vJoyConfig`, opens Zadig (`tools\zadig-2.9.exe`, pre-set to WinUSB): pick `0A7B D000` → Install Driver. Re-runs the check → `logs\step3-drivers.txt` |
| 4 | `4-capture.cmd` | no | Build + USB info + LED probe + live control dump → `logs\p1-*.txt` |
| 4b | `4b-calibrate.cmd` | no | 3 s hands off, then move every stick/lever/pedal to its limits → `config\calibration.toml` (restarts the bridge if installed) |
| 5 | `5-install-bridge.cmd` | no | Build `sbc-bridge`, copy exe + `config\calibration.toml` to `bin\`, add to Startup (hidden), start it → `logs\bridge.log` |
| — | `bridge-start.cmd` / `bridge-stop.cmd` | no | Start / cleanly stop the bridge (stop returns vJoy to neutral) |
| 6 | `6-uninstall-bridge.cmd` | no | Stop and remove from Startup |

`tools\zadig-2.9.exe` is not in git: download it from https://github.com/pbatard/libwdi/releases/tag/v1.5.1 (5,334,088 bytes, signed by Akeo Consulting).

Step 3b replaces the controller's driver. Undo: Device Manager → device → Uninstall device (tick "delete driver") → replug.
