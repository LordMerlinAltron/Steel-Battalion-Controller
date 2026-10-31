# Setup (double-click in order)

| Step | File / action | Admin | What it does |
|---|---|---|---|
| 1 | `1-check.cmd` | yes (UAC) | Read-only check → `logs\p0-check.txt` |
| 2 | `2-install-rust.cmd` | no | Rust GNU toolchain (no Visual Studio) → `logs\step2-rust.txt` |
| 3a | Install **vJoy 2.1.9.1** | yes | https://github.com/jshafer817/vJoy/releases/tag/v2.1.9.1 → `vJoySetup.exe`, reboot if asked |
| 3b | **Zadig** → WinUSB on the controller | yes | https://zadig.akeo.ie/ → Options ▸ List All Devices → pick `0A7B D000` (USB ID field) → target **WinUSB** → Replace/Install Driver |
| 4 | `4-capture.cmd` | no | Build + USB info + LED probe + live control dump → `logs\p1-*.txt` |

Step 3b replaces the controller's driver. Undo: Device Manager → device → Uninstall device (tick "delete driver") → replug.
