//! All fixes. Each one: read INI -> locate (signature or verified bytes) -> patch -> log.
use crate::{ini, logf, mem};
use mem::*;

static mut POOL: Option<Pool> = None;
fn pool() -> &'static mut Pool { unsafe { (*core::ptr::addr_of_mut!(POOL)).get_or_insert_with(|| Pool::new(0x10000)) } }

pub unsafe fn apply_all() {
    logf!("base {:#x}", base());
    let list: &[(&str, unsafe fn() -> Result<String, String>)] = &[
        ("FrameLimiter", fps_limiter),
        ("RenderDistanceSmallProps", render_slices),
        ("PushPhysicsFix", push_fix),
        ("Movement", movement),
        ("PivotTurn", pivot_turn),
    ];
    for (name, f) in list {
        match unsafe { f() } {
            Ok(m) => logf!("[ok]   {name}: {m}"),
            Err(e) => logf!("[skip] {name}: {e}"),
        }
    }
    crate::fixes_extra::apply_extra();
    crate::fixes3::apply();
    crate::fixes4::apply();
    crate::fixes5::apply();
    crate::fixes6::apply();
    crate::fixes7::apply();
    crate::fixes8::apply();
}

pub fn pool_f32(v: f32) -> usize { pool().put_f32(v) }
pub fn pool_ref() -> &'static mut Pool { pool() }

fn one(pat: &str) -> Result<usize, String> {
    let v = find_text(pat);
    if v.len() == 1 { Ok(v[0]) } else { Err(format!("signature matched {} times", v.len())) }
}

/// Engine frame limiter (FrameClock wait-for-frame flag). FPSLimit=0 -> limiter off (cap with DXVK/driver);
/// FPSLimit=N -> engine waits for 1/N s per frame.
unsafe fn fps_limiter() -> Result<String, String> {
    let limit = ini::f32v("Main", "FPSLimit", 0.0);
    if !ini::boolv("Main", "UnlockFPS", true) { return Err("disabled in ini".into()); }
    let p = one("D9 05 ?? ?? ?? ?? A2 ?? ?? ?? ?? D9 1D ?? ?? ?? ?? E8")?;
    let flag: u32 = unsafe { read(p + 7) };
    let target: u32 = unsafe { read(p + 13) };
    if flag.wrapping_sub(target) != 8 { return Err("structure check failed".into()); }
    if limit > 0.0 {
        let c = pool().put_f32(1.0 / limit);
        unsafe { write(p + 2, &(c as u32).to_le_bytes()); }
        Ok(format!("engine limiter set to {limit} FPS"))
    } else {
        unsafe { write(p + 6, &[0x90; 5]); }
        Ok("engine 60 FPS limiter removed".into())
    }
}

/// Per-model render-slice mask: allow every model in the first N depth slices (4 = ~100 m, 5 = far clip).
unsafe fn render_slices() -> Result<String, String> {
    let n = ini::i32v("Graphics", "MinRenderSlices", 4);
    if !(1..=5).contains(&n) || n <= 0 { return Err("disabled in ini".into()); }
    let p = one("81 CE ?? 01 00 00 C1 E6 05")
        .or_else(|_| one("81 CE E0 01 00 00"))?;
    let low = ((1u32 << n) - 1) as u8; // slices 0..n-1
    let cur = unsafe { read::<u8>(p + 2) };
    unsafe { write(p + 2, &[0xE0 | low]); }
    Ok(format!("render mask low bits {cur:#04x} -> {:#04x}", 0xE0 | low))
}

/// Human soft-separation: velocity = push / dt  ->  push / (1/60), so pushes are frame-rate independent.
unsafe fn push_fix() -> Result<String, String> {
    if !ini::boolv("Fixes", "PushPhysicsFix", true) { return Err("disabled in ini".into()); }
    let p = one("D8 74 24 30 D9 5C 24 30 D9 86 B0 01 00 00 D9 44 24 30")?;
    let c = pool().put_f32(1.0 / 60.0);
    let mut a = vec![0xD8, 0x35]; a.extend_from_slice(&(c as u32).to_le_bytes()); a.extend_from_slice(&[0x90, 0x90]);
    unsafe { write(p, &a); write(p + 14, &[0xD9, 0xC9, 0x90, 0x90]); }
    Ok("separation velocity uses fixed 1/60".into())
}

/// On-foot acceleration/deceleration constants (GOG build .data, verified by value).
unsafe fn movement() -> Result<String, String> {
    let mult = ini::f32v("Movement", "AccelerationMultiplier", 4.0);
    // (rva, original value)
    let k: [(usize, f32); 5] = [(0xD1A1D8, 10.0), (0xD1A1DC, 5.0), (0xD1A1C8, 10.0), (0xD1A1CC, 5.0), (0xD1A1E0, 8.0)];
    let b = base();
    for (rva, v) in k { if unsafe { read::<f32>(b + rva) } != v { return Err(format!("unexpected value at rva {rva:#x} (different build?)")); } }
    // accel/decel: scale; sprint accel 8 -> 8*mult capped at 30
    let scaled = [(0xD1A1D8, (10.0 * mult).min(60.0)), (0xD1A1DC, (5.0 * mult * 1.5).min(40.0)), (0xD1A1C8, (10.0 * mult).min(60.0)),
                  (0xD1A1CC, (5.0 * mult).min(40.0)), (0xD1A1E0, (8.0 * mult).min(40.0))];
    for (rva, v) in scaled { unsafe { write(b + rva, &v.to_le_bytes()); } }
    Ok(format!("accel x{mult}"))
}

/// WSHumanStateOnGround moving-state pivot (FUN_0055c4e0): when the stick/WASD direction differs from the
/// player's heading by more than 0.5*pi (.data 0x111A0BC, player only; NPCs use 0x111A0B4/B8), speed is reset to 0
/// and the human enters turn state 3 (stop, turn animation, start again). PivotTurnAngle in degrees; >=180 = never.
unsafe fn pivot_turn() -> Result<String, String> {
    let deg = ini::f32v("Movement", "PivotTurnAngle", 180.0);
    let b = base();
    if unsafe { read::<f32>(b + 0xD1A0BC) } != 0.5 { return Err("unexpected value at rva 0xd1a0bc (different build?)".into()); }
    if ini::boolv("Debug", "PivotCounter", false) {
        // count pivots: cave over `mov ecx,3` at 0x55C751
        let site = b + 0x15C751;
        if bytes_at(site, 5) == [0xB9, 3, 0, 0, 0] {
            let p = pool();
            let ctr = p.put_bytes(&[0; 4]);
            let at = p.put_bytes(&[0xCC; 24]);
            let mut v = vec![0xFF, 0x05]; v.extend((ctr as u32).to_le_bytes()); v.extend([0xB9, 3, 0, 0, 0, 0xE9]);
            v.extend((((site + 5) as isize - (at as isize + v.len() as isize + 4)) as i32).to_le_bytes());
            unsafe { write(at, &v); jmp(site, at, 5); }
            logf!("pivot counter at {ctr:#x}");
        }
    }
    if (deg - 90.0).abs() < 0.01 { return Err("90 (vanilla)".into()); }
    let v = if deg >= 180.0 { 1.01f32 } else { (deg / 180.0).max(0.1) };
    unsafe { write(b + 0xD1A0BC, &v.to_le_bytes()); }
    Ok(format!("player pivot-turn threshold {}", if deg >= 180.0 { "off (never stops to turn)".to_string() } else { format!("{deg} deg (vanilla 90)") }))
}
