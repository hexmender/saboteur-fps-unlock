# Camera smoothing & mouse input — research notes

Status: **camera spring patch is shipped as an experimental tool** (`tools/tension_patch.py`). The mouse path is mapped. Some residual "floatiness" is still unexplained (see Open questions).

Addresses refer to the GOG `Saboteur.exe` (image base `0x400000`). Decompilation was done with Ghidra 12.1 headless. The exe has many SecuROM-style *split functions* (prologue moved elsewhere and joined back with `jmp`), so functions had to be rebuilt by following `jmp` flows (`ghidra_scripts/Decomp.java`, `flow_` mode).

## Camera springs (`CameraMoveTension` / `CameraAimTension`)

Camera behaviour is data-driven. `CameraSettings` / `GroupCameraSettings` templates live in `GameTemplates.wsd`, which is embedded uncompressed in `France\loosefiles_BinPC.pack` (format: Mercenaries-Fan-Build notes, `docs/formats/gametemplates.md`).

| Property | Blueprint offset | Default (ctor `0x46e9c0`) |
|---|---|---|
| `CameraMoveTension` (vec3) | `+0xb0` | `1, 1, 1` |
| `CameraAimTension` (vec3) | `+0xc0` | `1, 1, 1` |
| `RotSpeed` | `+0xe0` | |
| `PitchSpeed` | `+0xe4` | |

The parser is the `CameraSettings` property switch around `0x46e350`. Its `this` sits at blueprint `+0x150`, so stored offsets show up as negative displacements there.

The spring is applied each frame in `FUN_004702b0` (position) and `FUN_00401fef` (yaw/pitch/roll). For each axis:

```
if (tension < 1)  spring(value, velocity, dt, k = tension / (1 - tension))   // FUN_00472bb0 / FUN_00472dd0
else              value = target, velocity = 0                               // no smoothing at all
```

So **any tension ≥ 1.0 means a rigid camera on that axis**. Retail data uses 0.9–0.999 for on-foot (jump, melee, climbing, ledge hang) and vehicle cameras, 0.8 for boats, 0.1 for the Puma armoured car, and 2.0 for turrets. `tension_patch.py` raises every component below 1.0 to 1.0, which touches 161 pairs in place without changing the file size.

Call chain: `WSGameCamera::Tick` `0x671ae0` → state 0 `0x672780` (layer-blend loop, up to 4 settings layers) → layer update via the virtual call at `+0x210` → transition `0x472630` → orbit/follow `0x405586`, which calls the `0x4702b0` springs for position and look-at, and `0x401fef` for aim.

`0x675ba0` is camera collision. It has its own exponential recovery when the camera pulls back out from a wall (`+0xae0`, factor at `0xf8b90c`).

## Mouse path

`FUN_00686960` (per-frame input poll, DirectInput8):

```
GetDeviceState(mouse, DIMOUSESTATE)
sens   = (optionSlider + 0.25) / 1.25            // slider at [0x120f4b8]+0x27018 / +0x2701c
axisX  = lX * 0.04 * sens ;  axisY = lY * 0.04 * sens
split into 4 half-axis "buttons" (positive parts) -> FUN_00684300 (press/hold/double-tap state, value passed through unchanged)
```

There is **no smoothing or acceleration in the input layer**. Actions come from `InputTemplates.txt` (`SOLDIER_TURN_LEFT/RIGHT` = action 4/5, `SOLDIER_LOOK_UP/DOWN` = 6/7). The player controller `FUN_00691e00` computes `turn = A(5) - A(4)` → `+0x2c` and `look = A(6) - A(7)` → `+0x28` via `FUN_00683d80` (action slot `+0x1aa8 + i*0x14`) and `FUN_006841e0` (raw float).

Because the value is "counts per frame", it behaves like a stick deflection. If the consumer multiplies by `RotSpeed * dt`, mouse turn speed would depend on frame rate. This has not been verified yet.

## Open questions

- The consumer of controller `+0x28/+0x2c` (human aim/turn code) has not been identified yet. It may apply a rate limit or clamp (stick-style `|v| ≤ 1` × `RotSpeed`), which would feel like mouse lag on fast flicks.
- Window-box / facade decorations pop in only a few metres away. This is **not** the render-slice table (`0x1120ad8`, 5 sets × 5 slices, picked by `SliceQuality`). Extending slice 3 from 100 m to 300 m had no visible effect, so the patch was reverted. The likely culprits are a small-object distance or screen-size cull, or the "High" streaming grid.
