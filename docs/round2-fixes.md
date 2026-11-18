# Round 2: LOD, movement, camera transitions, push physics, intro

All offsets are file offsets in the GOG `Saboteur.exe` unless the pack is named. Every patch verifies the original bytes first.

## Pop-in: render-slice masks
The scene is drawn in 5 depth slices, and each slice is its own pass. Every model has a 5-bit render mask (`WSModel+0x9c`) that says which slices it may draw in.
- The mask is built in `FUN_006394d0` from `ModelInfo.txt` (`RENDERSLICEn` means slices `0..n-1`).
- Models not listed in `ModelInfo.txt` get a mask from a bounding-radius fallback: under 0.3 → 2 slices, under 0.6 → 3, under 4 → 4, otherwise 5.

Window boxes (`RENDERSLICE3`) were therefore limited to slices 0–2. Slice 2 ends at about 20 m with SliceQuality 3, which is why they appeared "a few steps away".

Patch at `0x238875` (`or esi,0x1E0` → `0x1EF`): every model is allowed slices 0–3, up to about 100 m. Using `0xFF` instead allows all 5 slices, up to the far clip, at a higher FPS cost.

## WASD movement
On-foot movement is in `FUN_00561ba0` (case 7). Speed approaches its target speed at `rate*dt`. The rates are single-reader `.data` constants:

| Offset | Meaning | Old → new |
|---|---|---|
| `0xD18BD8` | run acceleration | 10 → 40 |
| `0xD18BDC` | run acceleration near max speed | 5 → 30 |
| `0xD18BC8` | deceleration | 10 → 40 |
| `0xD18BCC` | deceleration (alt state) | 5 → 20 |
| `0xD18BE0` | sprint acceleration | 8 → 30 |
| `0xD18BC4` | heading damper | 0.5 → 0 |

The heading damper is a per-frame lerp of the desired heading back toward the current facing, applied without dt. At 240 FPS it resisted turning about 4× more than intended.

## Camera: transition tensions are the steady-state smoothing
`FUN_00405586` takes its move and aim spring tensions from the active camera *transition* template. It lerps the start values (`+0x24`/`+0x34`) toward the end values (`+0x48`/`+0x58`) over `TransitionTime`, and the end values stay in force after the blend finishes.

- DefaultTransition and the other transitions end at 0.975–0.98, which is about 30 ms of lag on every mouse movement.
- All end tensions were raised to 1.0, which means rigid. Transitions still ease in.
- AutoCam (`FUN_005ac400`) applies a fixed fraction per frame with no dt, so it recentred about 4× harder at 240 FPS. It is disabled with `GlobalHumanParams.AutoCamBlendDuration = 0`.
- The pack edits ship as `mod/camera_pack_patch.json` (offset, old hex, new hex). It also includes the CameraSettings tension edits from round 1.

## Push physics
`FUN_00502be0` runs a soft separation every frame for humans closer than 0.5 m (`push = dir*(0.5-d)*0.35`). `FUN_00584310` then turns that push into a velocity with `|push| / dt` (VA `0x5845E2`). Because 35 % of the overlap is corrected every *frame*, at 240 FPS the velocity spike is 4× larger and NPCs fly off.

The patch divides by the 1/60 constant at `0xFB9920` instead (`D8 74 24 30 D9 5C 24 30` → `D8 35 20 99 FB 00 90 90`). The dropped reload at `0x5845F0` becomes `fxch` (`D9 44 24 30` → `D9 C9 90 90`), which keeps the FPU stack depth the same.

At 60 FPS the behaviour is identical to before. Below about 40 FPS, humans that overlap could over-correct.

## Intro
`Bink\saboteur.bik` (and `saboteur_de.bik`) are replaced with the game's own `Sab_Placeholder.bik`. The originals are kept as `.orig`.
