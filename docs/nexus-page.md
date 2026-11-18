# Draft Nexus Mods page

**Name:** SaboteurFix – High FPS, Input and Graphics Overhaul

**Summary (short):** Unlocks the 60 FPS cap and fixes everything that breaks above 60: mouse, physics, camera shake and timers. Also removes camera and movement lag, sharpens shadows, reduces pop-in, adds Hor+ FOV and borderless mode, makes the pause menu instant and skips the intros. It patches the game in memory, so the exe is never modified, and every fix can be toggled in an INI.

**Description:**

The Saboteur is locked to 60 FPS, and several of its systems are tied to frame rate. SaboteurFix is a `dinput8.dll` plugin, in the spirit of SilentPatch and Fusion Fix, built by reverse-engineering the engine. It removes the cap and fixes those systems one by one, so the game plays the same at 144 or 240 FPS as it does at 60.

[b]High frame rate[/b]
- 60 FPS limiter removed. Cap the frame rate with the bundled DXVK config or with your driver.
- Mouse look is frame-rate independent. Vanilla turned 4× slower at 240 FPS.
- NPC pushing and Havok character pushes fixed. NPCs no longer fly away like volleyballs.
- Camera shake, skid marks, ground snapping on stairs and impact effects all run on real time.
- Max frame time is capped, so hitches can't make physics explode.

[b]Responsiveness[/b]
- All camera spring smoothing removed (about 30 ms of lag on every mouse movement), and auto-recentre turned off.
- Snappier on-foot acceleration and turning.
- DXVK with a 1-frame queue and low-latency pacing.

[b]Graphics[/b]
- Shadow maps 2048² (vanilla 1024²), cascade switches pushed out, and small props cast shadows farther.
- Window boxes and facade details drawn to about 100 m (vanilla about 20 m).
- LOD switch at 400 m (vanilla 150 m). Characters stay detailed to 30–60 m (vanilla 14–30 m). Vehicles stay detailed to 90 m (vanilla 25–30 m). Foliage draw distance ×2.
- Far plane 2500 m, Hor+ FOV (fixes ultrawide) with a multiplier, water reflections at 2× resolution, 16× anisotropic filtering.

[b]Quality of life[/b]
- Straight to the main menu: no intro movie and no legal screen.
- Instant ESC menu (no re-streaming on every open) and UI animations 2× faster.
- Borderless fullscreen.
- Faster loading: high-resolution timer, and streaming threads spread across CPUs.

[b]Install[/b]
1. Launch the game once, save and quit.
2. Run Install.bat.
3. Tune SaboteurFix.ini and dxvk.conf.

Uninstall.bat restores everything.

[b]Compatibility[/b]
- Tested on the GOG version.
- The plugin finds its patch sites by signature or verified bytes and skips anything it can't match; see SaboteurFix.log.
- Should coexist with ReShade. It may conflict with other mods that patch the same code (for example Fusion Fix's FOV or FPS options); disable the overlapping features in one of them.

[b]Credits[/b]
- Reverse engineering: Ghidra, plus the notes-on-reversing-the-saboteur community repo (RTTI symbol map and formats).
- DXVK by doitsujin et al. (zlib license).
