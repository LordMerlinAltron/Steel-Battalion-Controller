# Control Map

Physical control → raw input → vJoy output → JoyToKey.
Physical reference: [controller-layout.png](controller-layout.png) (labels in the image are an Elite Dangerous profile; only panel positions are used here).

Status: 🟡 = one source, 🟢 = 2019 driver and SteelBattalion.NET agree, ✅ = confirmed on hardware (`sbc-capture watch` / `led-probe`).
LED id = vJoy button number for buttons 4–33 (see [protocol.md](protocol.md)).

## Axes

| vJoy axis | Raw field | Physical | Status (raw, 2026-11-01 capture) |
|---|---|---|---|
| X | AimX | Right block — aim stick left/right (friction-held, **no spring**) | ✅ 1.0k–64.4k, linear (middle of travel = center) |
| Y | AimY | Right block — aim stick fwd/back (friction-held, **no spring**) | ✅ 1.0k–64.4k, linear (middle of travel = center) |
| Z | Rotation | Left block — rotation lever (springs back) | ✅ rest ~0, −31.8k–31.4k |
| Rx | SightX | Left block — sight change stick X (springs back) | ✅ rest ~0, −28.4k–28.8k |
| Ry | SightY | Left block — sight change stick Y (springs back) | ✅ rest ~0, −27.6k–28.5k |
| Rz | Throttle | Right pedal | ✅ rest 0, –60.5k |
| Slider 0 | Clutch | Left pedal | ✅ rest 0.7k, –63.9k |
| Slider 1 | Brake | Middle pedal | ✅ rest 4.3k, –63.2k |

## Buttons (raw bits)

| vJoy | Byte.bit | Name | Physical | LED | Status |
|---|---|---|---|---|---|
| 1 | B0.0 | RightJoyMainWeapon | Aim stick — thumb lever | none | ✅ |
| 2 | B0.1 | RightJoyFire | Aim stick — trigger | none | ✅ mask 0x02 (alone, no bit 0) |
| 3 | B0.2 | RightJoyLockOn | Aim stick — thumb button | none | ✅ |
| 4 | B0.3 | Eject | Right block — covered button | id 4, toggle | ✅ |
| 5 | B0.4 | CockpitHatch | Right block — column, top | id 5, toggle | ✅ |
| 6 | B0.5 | Ignition | Right block — column, middle | id 6, toggle | ✅ |
| 7 | B0.6 | Start | Right block — column, bottom | id 7, toggle | ✅ |
| 8 | B0.7 | MultiMonOpenClose | Right block — multi monitor | id 8, toggle | ✅ |
| 9 | B1.0 | MultiMonMapZoomInOut | Right block — multi monitor | id 9, toggle | ✅ |
| 10 | B1.1 | MultiMonModeSelect | Right block — multi monitor | id 10, toggle | ✅ |
| 11 | B1.2 | MultiMonSubMonitor | Right block — multi monitor | id 11, toggle | ✅ |
| 12 | B1.3 | MainMonZoomIn | Right block — main monitor zoom | id 12, toggle | ✅ |
| 13 | B1.4 | MainMonZoomOut | Right block — main monitor zoom | id 13, toggle | ✅ |
| 14 | B1.5 | FunctionFSS | Center block — 3×3 grid | id 14, toggle | ✅ |
| 15 | B1.6 | FunctionManipulator | Center block — 3×3 grid | id 15, toggle | ✅ |
| 16 | B1.7 | FunctionLineColorChange | Center block — 3×3 grid | id 16, toggle | ✅ |
| 17 | B2.0 | Washing | Center block — 3×3 grid | id 17, toggle | ✅ |
| 18 | B2.1 | Extinguisher | Center block — 3×3 grid | id 18, toggle | ✅ |
| 19 | B2.2 | Chaff | Center block — 3×3 grid | id 19, toggle | ✅ |
| 20 | B2.3 | FunctionTankDetach | Center block — 3×3 grid | id 20, toggle | ✅ |
| 21 | B2.4 | FunctionOverride | Center block — 3×3 grid | id 21, toggle | ✅ |
| 22 | B2.5 | FunctionNightScope | Center block — 3×3 grid | id 22, toggle | ✅ |
| 23 | B2.6 | FunctionF1 | Center block — lower row | id 23, toggle | ✅ |
| 24 | B2.7 | FunctionF2 | Center block — lower row | id 24, toggle | ✅ |
| 25 | B3.0 | FunctionF3 | Center block — lower row | id 25, toggle | ✅ |
| 26 | B3.1 | WeaponCtrlMain | Center block — lower row | id 26, toggle | ✅ |
| 27 | B3.2 | WeaponCtrlSub | Center block — lower row | id 27, toggle | ✅ |
| 28 | B3.3 | WeaponCtrlMagazineChange | Center block — lower row | id 28, toggle | ✅ |
| 29 | B3.4 | Comm1 | Center block — top row | id 29, toggle | ✅ |
| 30 | B3.5 | Comm2 | Center block — top row | id 30, toggle | ✅ |
| 31 | B3.6 | Comm3 | Center block — top row | id 31, toggle | ✅ |
| 32 | B3.7 | Comm4 | Center block — top row | id 32, toggle | ✅ |
| 33 | B4.0 | Comm5 | Center block — top row | id 33, toggle | ✅ |
| 34 | B4.1 | LeftJoySightChange | Left block — sight stick press | none | 🟢 not pressed in capture |
| 35 | B4.2 | ToggleFilterControl | Left block — toggle switch | none | ✅ bounces 10–70 ms → debounced |
| 36 | B4.3 | ToggleOxygenSupply | Left block — toggle switch | none | ✅ bounces 10–70 ms → debounced |
| 37 | B4.4 | ToggleFuelFlowRate | Left block — toggle switch | none | ✅ bounces 10–70 ms → debounced |
| 38 | B4.5 | ToggleBufferMaterial | Left block — toggle switch | none | ✅ bounces 10–70 ms → debounced |
| 39 | B4.6 | ToggleVTLocation | Left block — toggle switch | none | ✅ bounces 10–70 ms → debounced |

## Synthesized buttons (bridge-generated)

| vJoy | Source | Behavior |
|---|---|---|
| 40–46 | Gear R, N, 1, 2, 3, 4, 5 | ✅ raw −2, −1, 1–5. Held while lever in position (none for ~10 ms between gears, raw 0); LED ids 35–41 keep the last gear |
| 47–62 | Tuner position 0–15 | ✅ held while dial at position (0 = 9 o'clock, 12 = 6 o'clock, 13–15 blank arc); counts up clockwise; a new position must hold 100 ms (`tuner_settle_ms`) |
| 63 | Tuner step + | pulse on each step up |
| 64 | Tuner step − | pulse on each step down |
| 65–69 | Toggles 35–39 switched ON | pulse; one per flip (80 ms debounce, `toggle_debounce_ms`) |
| 70–74 | Toggles 35–39 switched OFF | pulse |

vJoy device config: 8 axes, 80 buttons, 0 POV.

Axes are scaled with `sbc-calibration.toml`: spring-centered controls map rest → vJoy center; the aim stick (no spring) and pedals map min → max linearly. Make one with `setup\4b-calibrate.cmd`.

## LED behavior

| Mode | Applies to | Behavior |
|---|---|---|
| toggle | vJoy 4–33 | press → on, press again → off. Output to JoyToKey stays momentary. |
| gear | Gear LEDs 35–41 | `position` (current gear only), `progressive` (N…current), or `off` |
| none | vJoy 1–3, 34–39 | no LED hardware |
