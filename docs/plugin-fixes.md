# SaboteurFix plugin: every fix, with its mechanism

All addresses are VAs in the GOG `Saboteur.exe` (base `0x400000`). RVA = VA − 0x400000. Every patch is verified against the expected original bytes or values before it is written. Anything that doesn't match is skipped and written to `SaboteurFix.log`. Throughout, `dt` means the frame time in seconds.

## Frame rate and simulation
| Fix | Site | Mechanism |
|---|---|---|
| FPS unlock | init of FrameClock `0xF641A0` | `waitForFrame` flag write is NOPed, or `FPSLimit=N` repoints the target frame time to 1/N. |
| Mouse | `0x686AAC` (cave) | The mouse "stick" value is counts×0.04×sens per frame, and the aim code multiplies it by `dt` again, so at 240 FPS it turned 4× slower. The cave divides the 0.04 scale by 60×rawDt, so the angle per mouse count is constant at any FPS. The gamepad is untouched. |
| Push separation | `0x5845E2` | Human soft-separation computes velocity = overlap/dt. It is replaced by overlap/(1/60). |
| Havok character push | `0xB44D8E` (cave) | `hkpCharacterProxy::applySurfaceInteractions` subtracted projVel×mass on every step. That term is now scaled by min(stepDt×60, 1). |
| Movement | `.data 0x111A1C4..E0` | Run/sprint acceleration and deceleration about ×4. The per-frame heading damper (0.5, no dt) is set to 0. |
| Camera shake | `0x67B76B` (in place) and kicks `0x6783C8`/`0x678523` (caves) | The shake spring was stepped a fixed 0.05 s per frame. It is now time-based with s = min(dt×60, 1), so it stays stable. It also resets itself if a value becomes NaN or Inf. The kicks are scaled by √s. |
| 60 Hz frame counter | main loop `0x43A423` (cave) | `g_frame60 += round(acc + dt×60)`. Frame-count timers can read it instead of the raw frame counter. The same cave computes the per-frame ground-snap factor. |
| Ground snap | `0x584AF9` | The per-frame 0.4 snap becomes `1 − 0.6^(dt×60)`. It is precomputed in the main-loop cave; an in-place cave hung loading because of FPU stack depth. |
| Skid marks | `0x60E9C7` (cave) plus ages at `0x60E77E`/`0x60E993` | Fade becomes 0.6×dt instead of 0.01 per frame. Mark ages use `g_frame60`. |
| Impact-FX throttle | `0x92709C`/`0x927142` | The ring that skips repeated FX for 15 frames uses `g_frame60`. |
| dt cap (off by default) | `0xDB6FB6`/`0xDB6FC9` | This cap applies only to cutscenes and Bink movies; gameplay is already capped at 0.05 s. Lowering it can desync cutscene audio, so `MaxFrameTime=0.2` (vanilla) is the default. |
| Per-frame decays | trains `0x5FBEB9`/`0x5FBEC5`, leaves `0x6CF7C0`/`0x6CFC8E`, vehicle camera sway `0x67A04C..0x67A389` | Decays applied per frame without dt (0.999 / 0.99 / 0.98 / 0.93 / 0.1 / 0.9 / 0.8) are repointed to `c^(dt*60)` values that the main-loop cave computes every frame. |
| Leaves (full), race AI, move input | leaves `0x6CFC4A`/`0x6CFC94`/`0x6CF780`, race AI lane offset `0x8FB3D5`/`0x8FB3DD`, on-foot move-input smoothing `0x5BF630` | Leaf spin input is scaled by s and the applied angle by s, and normal drag becomes `1-0.5^s`. The race AI lane lerp and on-foot move-input lerp become exact s-frame solutions. |
| Speed FOV | `.data 0x111D54C/50` | The speed-FOV blend ratio ramps at 0.25/s up and 0.3/s down (vanilla 0.08/0.1). |
| FOV transitions | `fov_pack_patch.json` | Transition FOV tensions are raised to 0.99, so FOV changes settle in about 50 ms instead of about 250 ms. |
| Loading | `timeBeginPeriod(1)` | Streaming wait loops use `Sleep(1)`, which lasts ~15.6 ms without this. |
| Streaming threads | `0x987574` | Engine threads 5–7 (stream read, stream decompress, one job worker) were all pinned to the last logical CPU. They now get every CPU except CPU 0. |

## Graphics
| Fix | Site | Mechanism |
|---|---|---|
| Small-prop draw distance | `0x639673` (`or esi,0x1E0` → `0x1EF`) | The per-model render-slice mask. Decorations flagged RENDERSLICE3 were drawn only in depth slices 0–2 (~20 m). |
| Shadow maps | `.data 0x11D61B4/B8` and snap `0x1120EE4` | 1024 → 2048 for all 5 cascades. |
| Shadow cascades | slice set 3 `0x1120B8C` and minimap restore `0x43B83C` | Splits move from 1.5/5/20/100 m to 4/15/40/150 m. |
| Small-prop shadows | `0x63965D` | The fallback shadow mask for models not in ModelInfo is raised to cascades 0–3. |
| LOD | `.data 0x1114FB4`, foliage `0x63960D` | LOD1 switch 150 → 400 m; foliage 50 → 80 m. |
| Characters | `0x4F176E…`, ctor copy, `.data 0x111792C`, `0x111773C/40` | Character LOD 14/30 → 30/60 m, draw distance 105 → 200 m, shadow distance 25 → 50 m. |
| Draw distance | `0x642F8E` | The ClipRange 3 far plane 1000 → 2500 m. |
| FOV | `0xE0501A` (setter rewrite) and `0x43A4B1`/`0x43A8C1` | The engine stores the horizontal tan and derives the vertical one from it, which makes it Vert-. The rewrite keeps the vertical FOV of a reference aspect (Hor+). A dead `fadd [0]` becomes an FOV multiplier. |
| Water reflections | `.data 0x11C13D8/DC` | Reflection render target 512×128 → 1024×256. |
| Vehicles | `ModelInfo.txt` (installer) | `LODDIST25/30` → `LODDIST90` and `SHADOWSLICE3` → `SHADOWSLICE4`. |
| Foliage | `loosefiles_BinPC.pack` (installer) | RenderDist / MaxRenderDist ×2. |
| Anisotropic filtering | `dxvk.conf` | `d3d9.samplerAnisotropy = 16` |

## UI and quality of life
| Fix | Site | Mechanism |
|---|---|---|
| Intro and legal screen | `0x7B5F84` | The boot state machine's first transition goes straight to "done" instead of the movie, then legal (4.5 s), then done. |
| UI speed | `0x79E5AB` (cave) | `GFxMovieView::Advance(dt)` is called with dt×AnimationSpeed. |
| Instant ESC | `0x9BEEB2`, `0x7A4743`, `0x7A23F9`, `0x7A4723`, `0x7A23DE` | The PauseMenu, GammaCorrection and WorldMapTextures stream blocks were unloaded on every close, so every open re-streamed about 11 MB. The unload calls become `add esp,4`. |
| Borderless | plugin thread | With `Windowed 1`, the window becomes a WS_POPUP covering the monitor. |
| Camera smoothing | `loosefiles_BinPC.pack` (installer) | CameraSettings tensions and transition end tensions raised to 1.0. Tension ≥ 1 means a rigid camera axis (`FUN_004702b0`/`FUN_00401fef`). AutoCam is disabled. |

## Developer tools
`[Debug] InputInjection=1` hooks `IDirectInputDevice8::GetDeviceState` and merges keys and mouse deltas from `SaboteurFix_input.txt`. It was used for automated in-game testing: the scripts load a save, walk around, open ESC and take screenshots.
