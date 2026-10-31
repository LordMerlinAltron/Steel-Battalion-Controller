# SBC USB Protocol Contract

Device: Steel Battalion Controller (original Xbox), USB `VID 0x0A7B`, `PID 0xD000`.
Host driver on Windows: **WinUSB** (bound once with Zadig).

Status: ✅ verified on hardware · 🟢 two independent sources agree · 🟡 one source · ❓ unknown

Sources: [2019 driver] caosdoar/SteelBattalionDriver (GPLv3) · [SB.NET] dmanning23/SteelBattalion.NET (Joseph Coutcher, 2010; LGPL/GPL).
Only protocol facts are recorded here. No code from either project is used.

## Setup 🟡

| Item | Value | Source |
|---|---|---|
| Configuration | 1 | SB.NET |
| Interface | 0 | SB.NET, 2019 driver |
| IN endpoint | `0x82`, interrupt | SB.NET |
| OUT endpoint | `0x01`, interrupt | SB.NET |

`sbc-usb` reads endpoint addresses from descriptors; `sbc-capture info` prints them.

## Input report (IN) — 26 bytes 🟢

Little-endian.

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0–1 | 2 | — | pad | ignored |
| 2 | 1 | u8 | Buttons0 | buttons 1–8 |
| 3 | 1 | u8 | Buttons1 | buttons 9–16 |
| 4 | 1 | u8 | Buttons2 | buttons 17–24 |
| 5 | 1 | u8 | Buttons3 | buttons 25–32 |
| 6 | 1 | u8 | Buttons4 | bits 0–6 → buttons 33–39, bit 7 unused |
| 7 | 1 | — | pad | ignored |
| 8 | 2 | u16 | AimX | aim stick X |
| 10 | 2 | u16 | AimY | aim stick Y |
| 12 | 2 | i16 | Rotation | rotation lever |
| 14 | 2 | i16 | SightX | sight change stick X |
| 16 | 2 | i16 | SightY | sight change stick Y |
| 18 | 2 | u16 | Clutch | **left** pedal 🟢 |
| 20 | 2 | u16 | Brake | **middle** pedal 🟢 |
| 22 | 2 | u16 | Throttle | **right** pedal 🟢 |
| 24 | 1 | u8 | Tuner | **low nibble only** (0–15); high nibble is not tuner data 🟡 |
| 25 | 1 | i8 | Gear | R=-2, N=-1, 1..5 inferred from 2019 driver ❓ |

Open question: SB.NET treats the aim-stick trigger as mask `0x03` on byte 2 (bits 0+1); the 2019 driver uses `0x02`. Check with `sbc-capture watch`.

## LED output report (OUT) — 34 bytes 🟡

- No header. Host keeps full LED state and rewrites the whole report on change.
- LED id `n` → byte `n / 2`. Even `n` = low nibble, odd `n` = high nibble.
- Brightness per LED: 0 (off) … 15 (full).

| LED id | Control |
|---|---|
| 0–3 | none |
| 4–33 | button with the same vJoy number (Eject … Comm5) |
| 34 | none |
| 35–41 | Gear R, N, 1, 2, 3, 4, 5 |
| 42–67 | none known |

No LEDs: aim-stick buttons (vJoy 1–3), sight-stick press (34), toggle switches (35–39).

Verify with `sbc-capture led-probe` (walks every id, prints the expected control).
