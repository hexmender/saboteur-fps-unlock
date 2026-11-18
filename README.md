# The Saboteur FPS Unlock: uncapped FPS, no input lag (GOG)

> **GOG version only.**

I vibe-coded this in one evening. I've never played this game. I got a new 240 Hz monitor, wanted more than 60 FPS, looked for a fix, found nothing, so I asked Claude to make one.

I didn't really test it in the game. In one evening I put together a way to get past 60 FPS. I also removed the intro and changed the character controls: no smoothing, no input delay. WASD responds instantly now, which feels great to me. I'm used to GTA controls, so I tried to get them roughly GTA-like and removed all the smoothing. The game seems fun.

There are bugs. At the very least you can shove pedestrians around like a tank, and there are probably other glitches. This is a raw, just-for-fun mod. I probably won't keep working on it, though I might push an update after I finish the game. I'm not a fan of this game; this is just AI slop, so don't take it as me claiming to be some modder.

I did a quick pass and added a bunch of small fixes on top. If anyone wants to polish it further, that would be awesome: fork it, fix stuff, send PRs.

<!-- demo video: drag SaboteurFix_demo.mp4 here in the GitHub editor -->

## What it is

A raw fix pack. It is a `dinput8.dll` plugin that patches the game **in memory** at startup, so `Saboteur.exe` is never modified. Every fix can be toggled and tuned in `SaboteurFix.ini`, and each one is located by signature or verified bytes before it is applied. Anything that doesn't match is skipped and listed in `SaboteurFix.log`.

Tested on the **GOG** release.

## What it does

**Frame rate and timing**
- Removes the engine's 60 FPS limiter. The engine already uses a variable timestep. Cap the frame rate in `dxvk.conf` or with your driver.
- **Mouse:** vanilla mouse look multiplied the per-frame mouse delta by dt, so at 240 FPS it turned 4× slower. Turning now feels the same at any frame rate.
- **NPC pushing:** human soft-separation velocity uses a fixed 1/60 step, so NPCs are no longer launched at high FPS.
- **Movement:** removes the per-frame heading damper, which got 4× stronger at 240 FPS. Run acceleration and deceleration are about 4× faster, which removes the floaty WASD feel.
- **Skid marks:** they fade over real time instead of per frame.
- **Loading:** `timeBeginPeriod(1)`, so the Sleep-based loading loops really sleep 1 ms instead of about 15.6 ms.

**Camera** (data patch)
- All camera spring smoothing removed: the CameraSettings tensions and the transition end tensions. These caused about 30 ms of lag on every mouse movement.
- Auto-recentre off.

**Graphics**
- **Shadows:**
  - shadow maps 2048² instead of 1024² (all 5 cascades);
  - cascade switches pushed from 1.5 / 5 / 20 / 100 m to 4 / 15 / 40 / 150 m;
  - small props cast shadows farther.
- **Pop-in:**
  - small props such as window boxes and facade details draw to about 100 m instead of about 20 m;
  - mesh LOD switch at 400 m instead of 150 m;
  - foliage LOD at 80 m instead of 50 m, and foliage draw distance ×2 (data patch);
  - characters keep full detail to 30–60 m instead of 14–30 m, are drawn to 200 m instead of 105 m, and their shadows reach 50 m instead of 25 m.
- **More pop-in options** (`[World]` in the ini, all off or vanilla-safe by default):
  - `GrassDistance`: grass draw distance (vanilla 50 m, default 120 m);
  - `DetailObjectDistanceScale`: placed grass and detail objects fade N times farther;
  - `PedestrianRadius` / `PedestrianSpawnRadius`: people exist and spawn farther away;
  - `TrafficRadius`: roads spawn traffic farther away (up to 500 m).
- **Draw distance:** far plane at 2500 m instead of 1000 m.
- **FOV:** Hor+ FOV, which fixes ultrawide, plus an FOV multiplier (default ×1.1).
- **Water reflections:** render target at 2× resolution.
- **Street life:** traffic density ×1.5 and pedestrian caps ×1.5.
- **Texture filtering:** 16× anisotropic filtering (DXVK).

**Quality of life**
- Skips the startup movie and the legal screen, so you boot straight to the menu.
- Pause menu and UI animations run 2× faster.
- Borderless fullscreen (with `Windowed 1`).
- DXVK (D3D9 → Vulkan) with a low-latency config: no render-ahead queue, VSync off, FPS cap.

## Requirements
- The Saboteur, **GOG** version. Other releases have a different exe: the plugin then skips the patches it can't verify, which you can see in `SaboteurFix.log`.
- Windows 10/11 and a Vulkan-capable GPU (for DXVK). Without DXVK, run the installer with `-NoDxvk`.

## Max-quality preset
If you don't care about FPS, set these in `SaboteurFix.ini`:
```
MinRenderSlices=5
ShadowMapSize=8192
Cascade0Far=5
Cascade1Far=20
Cascade2Far=60
Cascade3Far=250
SmallPropShadowCascades=5
LODDistance=1000
GrassDistance=150
DetailObjectDistanceScale=2.5
PedestrianRadius=200
PedestrianSpawnRadius=140
TrafficRadius=450
```

## Install
1. Launch the game once, start or load a game, save and quit. This creates `config.ini`.
2. Extract the zip anywhere and run **`Install.bat`**.
3. Tune `SaboteurFix.ini` and `dxvk.conf` (set `d3d9.maxFrameRate` to your refresh rate).

`Uninstall.bat` removes everything and restores every backup.

Installer flags: `-NoDxvk -NoCamera -NoFoliage -NoConfig -ResetIni -GameDir <path>`.

## Building from source
Install Rust with the `i686-pc-windows-msvc` target, then run `build-release.ps1 -Version x.y.z`. This builds `dist/SaboteurFix-x.y.z.zip` and downloads DXVK.

## Known issues
- AMD RDNA4 (RX 9000 series): DXVK 3.x's descriptor-heap path can hang the GPU. The bundled `dxvk.conf` turns it off with `dxvk.enableDescriptorHeap = False`.
- `StreamingDistance` and `StreamingDetailDistance` are experimental and can crash the game. Keep them at vanilla values.
- `CameraShakeFPSFix` is experimental and off by default: it currently breaks the main-menu camera.
- The NPC "volleyball" shove can still happen when an NPC is in a ragdoll or dynamic state. A fix for the Havok momentum term is planned.

## Credits
- [DXVK](https://github.com/doitsujin/dxvk) (zlib license), bundled `d3d9.dll`.
- [notes-on-reversing-the-saboteur](https://github.com/Mercenaries-Fan-Build/notes-on-reversing-the-saboteur): symbol map and reversing notes.

## Docs
- [docs/how-it-works.md](docs/how-it-works.md): FPS limiter
- [docs/camera-and-input.md](docs/camera-and-input.md): camera and input
- [docs/round2-fixes.md](docs/round2-fixes.md): LOD, movement, push physics
- `plugin/src/*.rs`: every patch, with its addresses and the reasoning behind it

## Credits
- Reverse engineering and the plugin: this repo, built with Ghidra and the community notes below.
- [notes-on-reversing-the-saboteur](https://github.com/Mercenaries-Fan-Build/notes-on-reversing-the-saboteur): RTTI symbol map and format notes.
- [DXVK](https://github.com/doitsujin/dxvk) (zlib license), bundled.
