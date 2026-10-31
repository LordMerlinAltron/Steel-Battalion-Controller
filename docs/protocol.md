# SBC USB Protocol Contract

Device: Steel Battalion Controller (original Xbox), USB `VID 0x0A7B`, `PID 0xD000`.
Host driver on Windows: **WinUSB** (bound once with Zadig).

Status: ✅ verified on hardware · 🟢 two or more sources agree · 🟡 one source · ❓ unknown

## Sources

| Tag | Source | What it gave us |
|---|---|---|
| [WIKI] | [xboxdevwiki — Xbox Input Devices](https://xboxdevwiki.net/Xbox_Input_Devices) | USB/XID descriptors, endpoint sizes, input + output report tables, tuner and gear semantics |
| [SB.NET] | [SteelBattalion.NET](https://github.com/dmanning23/SteelBattalion.NET) (Joseph Coutcher, 2010; LGPL/GPL) | Endpoints, LED nibble ids |
| [2019] | [caosdoar/SteelBattalionDriver](https://github.com/caosdoar/SteelBattalionDriver) (GPLv3) | Input struct as 16-bit fields |

Only protocol facts are recorded. No code from any source is used.

## USB descriptors 🟢 [WIKI, SB.NET]

| Item | Value |
|---|---|
| USB | 1.10, bus powered, 500 mA, EP0 max packet 8 |
| Configuration / interface | 1 / 0 |
| Interface class | `0x58` (88, Xbox) / subclass `0x42` (66) / protocol 0 |
| IN endpoint | `0x82`, interrupt, 32 bytes, interval 4 |
| OUT endpoint | `0x01`, interrupt, 32 bytes, interval 4 |

The IN endpoint **NAKs when nothing changed**: reports arrive only on input change.
`sbc-bridge` handles this by re-mapping the last state on read timeout (pulse buttons still expire).

## XID descriptor 🟡 [WIKI]

Vendor request: `bmRequestType 0xC1`, `bRequest 6`, `wValue 0x4200`, `wIndex 0`, `wLength 16`.

| Field | Value |
|---|---|
| bLength / bDescriptorType | `0x10` / `0x42` |
| bcdXid | `0x0100` |
| bType / bSubType | `0x80` (Steel Battalion) / `0x01` |
| bMaxInputReportSize | `0x1A` = **26** |
| bMaxOutputReportSize | `0x16` = **22** |

`sbc-capture info` and `sbc-bridge` read this at connect. The bridge disables LEDs if the device reports an output size other than 22.

## Input report (IN) — 26 bytes 🟢

Little-endian.

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| 0–1 | 2 | — | header | ignored |
| 2 | 1 | u8 | Buttons0 | buttons 1–8 |
| 3 | 1 | u8 | Buttons1 | buttons 9–16 |
| 4 | 1 | u8 | Buttons2 | buttons 17–24 |
| 5 | 1 | u8 | Buttons3 | buttons 25–32 |
| 6 | 1 | u8 | Buttons4 | bits 0–6 → buttons 33–39; bit 7 unnamed |
| 7 | 1 | — | pad | unnamed |
| 8 | 2 | u16 | AimX | 0 = left, 0xFFFF = right |
| 10 | 2 | u16 | AimY | 0 = top, 0xFFFF = bottom |
| 12 | 2 | i16 | Rotation | −0x7FFF left … 0x7FFF right |
| 14 | 2 | i16 | SightX | −0x7FFF left … 0x7FFF right |
| 16 | 2 | i16 | SightY | −0x7FFF top … 0x7FFF bottom |
| 18 | 2 | u16 | Clutch | left pedal |
| 20 | 2 | u16 | Brake | middle pedal |
| 22 | 2 | u16 | Throttle | right pedal |
| 24 | 1 | u8 | Tuner | low nibble only; high nibble unnamed |
| 25 | 1 | i8 | Gear | R = −2, N = −1, 1st–5th = 1–5, **0 = between gears** |

Offsets note: WIKI/SB.NET list the axes at odd offsets 9, 11, … 23 because SB.NET reads only the **high byte** of each
little-endian u16. That also explains WIKI's apparent overlap of "RightPedal at 23 with mask 0xFFFF" and the tuner at 24.
The [2019] driver reads the full u16 at 8, 10, … 22. Both agree; this crate uses the full 16 bits.

Tuner (WIKI): position 0 = 9 o'clock, 12 = 6 o'clock; 13–15 are the blank arc between 6 and 9 o'clock.

Open question: SB.NET masks the aim-stick trigger as `0x03` on byte 2; WIKI and [2019] list bit 1 only. Check with `sbc-capture watch`.

## LED output report (OUT) — 22 bytes 🟢 [WIKI layout, SB.NET nibble ids]

| Byte | Low nibble (`0x0F`) | High nibble (`0xF0`) |
|---|---|---|
| 0 | report id — always `0x00` | |
| 1 | length — always `0x16` | |
| 2 | EmergencyEject (id 4) | CockpitHatch (id 5) |
| 3 | Ignition | Start |
| 4 | OpenClose | MapZoomInOut |
| 5 | ModeSelect | SubMonitorModeSelect |
| 6 | MainMonitorZoomIn | MainMonitorZoomOut |
| 7 | ForecastShootingSystem | Manipulator |
| 8 | LineColorChange | Washing |
| 9 | Extinguisher | Chaff |
| 10 | TankDetach | Override |
| 11 | NightScope | F1 |
| 12 | F2 | F3 |
| 13 | MainWeaponControl | SubWeaponControl |
| 14 | MagazineChange | Comm1 |
| 15 | Comm2 | Comm3 |
| 16 | Comm4 | Comm5 (id 33) |
| 17 | — (id 34) | GearR (id 35) |
| 18 | GearN | Gear1 |
| 19 | Gear2 | Gear3 |
| 20 | Gear4 | Gear5 (id 41) |
| 21 | unused | |

- LED id `n` → byte `n / 2`, even = low nibble, odd = high nibble. Ids 4–33 equal the vJoy button numbers.
- Brightness 0 (off) … 15 (full). The host keeps the full state and rewrites the whole report on change.
- No LEDs: aim-stick buttons (vJoy 1–3), sight-stick press (34), toggle switches (35–39).
- Earlier revisions of this crate sent SB.NET's 34-byte buffer with no header. That exceeded the 32-byte OUT packet and omitted the length byte. Fixed after cross-checking WIKI.

Verify with `sbc-capture led-probe` (walks ids 4–41 and prints the expected control).
