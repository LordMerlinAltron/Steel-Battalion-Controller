# JoyToKey setup

The bridge feeds **vJoy device 1**. JoyToKey sees it as a normal joystick.
Button numbers below match [../docs/control-map.md](../docs/control-map.md).

## One-time JoyToKey settings
1. **Settings → Options**: "Number of joysticks to configure" must include the vJoy device.
   Press a controller button with "Highlight pressed buttons" on to find which JoyToKey tab is vJoy.
2. **Settings → Stick/POV**: choose **"Show all axes (8 way + POV x 4)"** so all 8 axes appear.
3. Button count: JoyToKey 6.2+ supports 128 buttons; we use up to 74.

## What to bind
| vJoy | Control | Tip |
|---|---|---|
| Axis X / Y | Aim stick | mouse movement, or arrow keys |
| Axis Z | Rotation lever | |
| Axis Rx / Ry | Sight-change stick | |
| Axis Rz | Right pedal (throttle) | key held past a threshold |
| Slider / Dial | Left pedal (clutch) / middle pedal (brake) | |
| 1–39 | Physical buttons and toggle switches | toggle switches hold while ON |
| 40–46 | Gear R, N, 1–5 | held while in that gear |
| 47–62 | Tuner position 0–15 | held at that position |
| 63 / 64 | Tuner step + / − | one tap per click of the dial |
| 65–69 | Toggle switch 1–5 flipped **on** | one tap — best for toggle keys |
| 70–74 | Toggle switch 1–5 flipped **off** | one tap |

Button LEDs (4–33) toggle on each press independently of JoyToKey. Hold **Start** for 2 s to turn all LEDs off.

Save your profile's `.cfg` (JoyToKey's profile folder) into this directory to keep it in git.
