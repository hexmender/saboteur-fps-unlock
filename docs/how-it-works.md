# How the 60 FPS limiter was found and removed

Target: `Saboteur.exe`, GOG build, 32-bit PE, TimeDateStamp 2009-12-11, image base `0x00400000`,
SHA-256 `628CDBB64A5317FEF66D3C2FE87AB063B0C659DFC92F5882B85828F90568E4EF`.
The binary is unpacked. A `.secu` section (SecuROM residue) is present but the code is plain x86.

## 1. Symptoms

- In fullscreen, the game rejects any `RefreshRate` above 60 in `config.ini` and silently falls back to 800×600, even though the monitor supports 120 Hz at 1440p.
- In windowed mode (`Windowed 1`) the swapchain follows the desktop (240 Hz), VSync is off and DXVK's cap is raised, but the game **still sits at exactly 60 FPS**.
  → The limit is inside the engine, not in the presentation layer.

## 2. Finding the limiter

Engine-side limiters almost always look like this: read a high-resolution timer, compare it to a target frame time, then `Sleep` until the target is reached. So the search started from the imports:

| Import | IAT |
|---|---|
| `QueryPerformanceCounter` | `0xF71140` |
| `QueryPerformanceFrequency` | `0xF711EC` |
| `Sleep` | `0xF7112C` |

Scanning `.text` for `call [0xF71140]` and `call [0xF7112C]` turns up one small class where both sit next to each other, around `0xDB6C30–0xDB6E40`.

### Timer class

```
0xDB6C30  Timer::Timer()
            scale = 1.0f                       ; +0x1C
            QueryPerformanceFrequency(&freq)   ; +0x10
            QueryPerformanceCounter(&start)    ; +0x00, +0x08
            dt = scale * 0.016                 ; +0x18 (initial guess)

0xDB6C90  Timer::Tick(bool wait, float targetSeconds)
            elapsed = (QPC() - last) / freq
            if (wait && targetSeconds > elapsed) {
                do {
                    if (targetSeconds - elapsed > 0.001) Sleep(1);
                    elapsed = (QPC() - last) / freq;
                } while (elapsed < targetSeconds);
            }
            dt   = elapsed * scale              ; +0x18
            last = now
```

`Tick` is the busy-wait/sleep loop. It only waits when `wait` is true.

### Frame clock

`Tick` has exactly two callers, both inside `0xDB6F10`. That function belongs to an object holding two `Timer`s (at `+0x00` and `+0x20`) plus settings:

| Offset | Meaning | Initial value |
|---|---|---|
| `+0x18` | current `dt` (global `0x14E1C68`, read by hundreds of functions) | — |
| `+0x44` | max `dt` clamp | `0.05` |
| `+0x48` | min `dt` clamp | `0.001` |
| `+0x4C` | **target frame time** | `1/60` (`0x3C888889`, const at `0xFB9920`) |
| `+0x50` | previous `dt` | `1/30` |
| `+0x54` | **wait-for-frame flag** (passed as `wait` to `Tick`) | `1` |
| `+0x55` | clamp enabled | `1` / set per frame by main loop |

```
0xDB6F10  FrameClock::Update()
            prevDt = dt
            timer0.Tick(this->waitForFrame, this->targetFrameTime)
            timer1.Tick(this->waitForFrame, this->targetFrameTime)
            if (clampEnabled) dt = clamp(dt, minDt, maxDt)
```

There is one global instance at `0x14E1C50`. The main loop calls `FrameClock::Update()` once per frame at `0x43A95E`, right after the render/present work.

### Where the 1/60 and the flag come from

The global instance is built by a static initializer at `0xF641A0`:

```
0xF641F1  D9 05 20 99 FB 00     fld   dword ptr [0xFB9920]   ; 1/60
0xF641F7  A2 A4 1C 4E 01        mov   byte ptr [0x14E1CA4], al ; waitForFrame = 1
0xF641FC  D9 1D 9C 1C 4E 01     fstp  dword ptr [0x14E1C9C]  ; targetFrameTime = 1/60
0xF64202  E8 ...                call  atexit
```

A cross-reference search shows that **nothing else writes** `0x14E1C9C` or `0x14E1CA4`. They are set once at startup and never touched again.

## 3. The patch

`0x14E1CA4` lies in the zero-initialized tail of `.data` (past the section's raw size), so if the `mov` never runs, the flag stays `0`:

```
file offset 0xB633F7   A2 A4 1C 4E 01   ->   90 90 90 90 90
```

With `waitForFrame == 0`, `Timer::Tick` skips the sleep loop. The game then renders as fast as possible and `dt` simply gets smaller. The 1 ms / 50 ms clamps stay in place, and at 240 FPS `dt ≈ 4.2 ms`, well inside them.

Patched SHA-256: `7339A0B7AE66CF718B52E4CE53805E6DA610CA942E0D79FC7299E417137A92DB`.

### Why this is safe(ish)

- Simulation is driven by the variable `dt` at `0x14E1C68`, which is read by roughly 90 call sites (AI, vehicles, camera, animation…). The engine was never fixed-step at 60.
- Only the *wait* is removed. The timer, the dt measurement and the clamping are untouched.
- The `1/60` constant itself (`0xFB9920`) is **not** modified, because five other code sites reference it (one sits next to the `WSPhysicsVehicle*` functions, so vehicle physics probably uses a fixed 1/60 sub-step).

### Signature (for other builds)

The patcher doesn't rely only on the hash. It searches for:

```
D9 05 ?? ?? ?? ??  A2 [flag:4]  D9 1D [target:4]  E8
```

and accepts the match only if it is **unique** and `flag == target + 8` (field offsets `+0x54` / `+0x4C`). On the GOG build there is exactly one hit.

## 4. Input lag

Separately from the FPS cap, the D3D9 runtime let the CPU queue several frames ahead of the GPU, which at 60 FPS makes the mouse feel very floaty. DXVK fixes this:

```
d3d9.maxFrameLatency = 1      # no render-ahead queue
d3d9.presentInterval = 0      # force VSync off
dxvk.latencySleep    = True   # DXVK low-latency pacing
d3d9.maxFrameRate    = 240    # cap now that the engine limiter is gone
```

## 5. Fullscreen refresh-rate check (not patched)

In fullscreen, the game rejects refresh rates above 60 and falls back to 800×600. Windowed mode avoids that check entirely: the swapchain uses the desktop refresh rate, so this was not patched. A future patch could find that validation, probably in the display-mode enumeration code around `Direct3DCreate9` (IAT `0xF7146C`).

## Tools

- Python 3 + `pefile` + `capstone`. Scripts are in [`../tools`](../tools).
- RTTI / symbol names from [notes-on-reversing-the-saboteur](https://github.com/Mercenaries-Fan-Build/notes-on-reversing-the-saboteur) (`data/symbol_map/pc_symbol_map.tsv`), which covers the same GOG exe.
