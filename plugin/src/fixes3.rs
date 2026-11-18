//! Round-3 fixes: world detail, FOV, high-FPS camera shake / skid marks, intro skip.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}

fn hx(s: &str) -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect() }
fn b() -> usize { base() }
fn rdf(rva: usize) -> f32 { unsafe { read::<f32>(b() + rva) } }

unsafe fn swap(rva: usize, old: &str, new: &[u8]) -> Result<(), String> {
    let o = hx(old);
    let cur = bytes_at(b() + rva, o.len());
    if cur != o { return Err(format!("rva {rva:#x}: {} != expected", hex(&cur))); }
    unsafe { write(b() + rva, new); }
    Ok(())
}

/// Repoint an "fld/fadd/fmul dword [abs]" (2-byte opcode + abs32) to a fresh constant `v`, optionally changing the opcode.
unsafe fn repoint(rva: usize, opcode: [u8; 2], old_abs_rva: usize, new_opcode: [u8; 2], v: f32) -> Result<(), String> {
    let mut o = opcode.to_vec(); o.extend_from_slice(&((b() + old_abs_rva) as u32).to_le_bytes());
    let cur = bytes_at(b() + rva, 6);
    if cur != o { return Err(format!("rva {rva:#x}: {} != {}", hex(&cur), hex(&o))); }
    let c = super::fixes::pool_f32(v);
    let mut n = new_opcode.to_vec(); n.extend_from_slice(&(c as u32).to_le_bytes());
    unsafe { write(b() + rva, &n); }
    Ok(())
}

unsafe fn setf(rva: usize, expect: f32, v: f32) -> Result<(), String> {
    let c = rdf(rva);
    if c != expect { return Err(format!("rva {rva:#x} = {c}, expected {expect}")); }
    unsafe { write(b() + rva, &v.to_le_bytes()); }
    Ok(())
}

pub fn apply() {
    unsafe {
        run("SkipIntroAndLegal", skip_intro());
        run("LODDistance", lod_distance());
        run("FoliageLOD", foliage_lod());
        run("PerModelLODScale", per_model_lod());
        run("HumanDetail", human_detail());
        run("DrawDistance", draw_distance());
        run("HorPlusFOV", hor_plus());
        run("FOVMultiplier", fov_mult());
        run("WaterReflections", water());


    }
}

/// Boot state machine: -1 -> 1 (saboteur.bik) -> 4 (legal 4.5 s) -> 5. Jump straight to 5 (or 4 = keep legal screen).
unsafe fn skip_intro() -> Result<String, String> {
    if !ini::boolv("UI", "SkipIntro", true) { return Err("disabled in ini".into()); }
    let keep_legal = ini::boolv("UI", "KeepLegalScreen", false);
    let new = if keep_legal { "b8040000009090" } else { "b8050000009090" };
    unsafe { swap(0x3B5F84, "8d048501000000", &hx(new))?; }
    Ok(if keep_legal { "startup movie skipped".into() } else { "startup movie + legal screen skipped".into() })
}

unsafe fn lod_distance() -> Result<String, String> {
    let d = ini::f32v("World", "LODDistance", 400.0);
    if d <= 150.0 { return Err("vanilla (ini)".into()); }
    unsafe { setf(0xD14FB4, 150.0, d)?; }
    Ok(format!("mesh LOD1 switch 150 -> {d} m"))
}

unsafe fn foliage_lod() -> Result<String, String> {
    let d = ini::f32v("World", "FoliageLODDistance", 80.0);
    if d <= 50.0 { return Err("vanilla (ini)".into()); }
    unsafe { repoint(0x23960D, [0xD9, 0x05], 0xB7D3A8, [0xD9, 0x05], d)?; }
    Ok(format!("foliage LOD 50 -> {d} m"))
}

/// Models with their own LOD distance under 100 m (FOLIAGE = trees/bushes 50 m, ModelInfo LODDIST) switch to the
/// low-poly LOD at that distance instead of the global one. Scale it: cave over `fld [ebp+0xa4]; fmul st0,st0` @0x48C7BE.
unsafe fn per_model_lod() -> Result<String, String> {
    let k = ini::f32v("World", "PerModelLODScale", 3.0).clamp(1.0, 10.0);
    if k <= 1.0 { return Err("vanilla (ini)".into()); }
    let site = b() + 0x8C7BE;
    if bytes_at(site, 8) != hx("d985a4000000dcc8") { return Err("site mismatch".into()); }
    let pool = super::fixes::pool_ref();
    let c = pool.put_f32(k);
    let at = pool.put_bytes(&[0xCC; 32]);
    let mut v = hx("d985a4000000d80d"); v.extend((c as u32).to_le_bytes()); v.extend(hx("dcc8"));
    let here = at + v.len();
    v.push(0xE9); v.extend((((site + 8) as isize - (here as isize + 5)) as i32).to_le_bytes());
    unsafe { write(at, &v); jmp(site, at, 8); }
    Ok(format!("per-model LOD distances x{k} (trees/foliage, ModelInfo LODDIST)"))
}

/// Human mesh LOD distances (two setters + inlined copy in the human ctor), draw and shadow distances.
unsafe fn human_detail() -> Result<String, String> {
    let l1 = ini::f32v("World", "HumanLOD1Distance", 30.0);
    let l1i = ini::f32v("World", "HumanLOD1DistanceImportant", 60.0);
    let l2 = ini::f32v("World", "HumanLOD2Distance", 60.0);
    let l2i = ini::f32v("World", "HumanLOD2DistanceImportant", 120.0);
    let draw = ini::f32v("World", "HumanDrawDistance", 200.0);
    let shadow = ini::f32v("World", "HumanShadowDistance", 50.0);
    // (rva, original constant rva, new value)
    let sites = [(0xF176E, 0xB9EB7C, l1), (0xF177A, 0xB7B134, l1i), (0xF17CE, 0xB7B134, l2), (0xF17DA, 0xB9EB84, l2i),
                 (0x10B1C4, 0xB9EB7C, l1), (0x10B1D0, 0xB7B134, l1i), (0x10B211, 0xB7B134, l2), (0x10B21D, 0xB9EB84, l2i)];
    for &(rva, old, _) in &sites {
        let mut o = vec![0xD9, 0x05]; o.extend_from_slice(&((b() + old) as u32).to_le_bytes());
        if bytes_at(b() + rva, 6) != o { return Err(format!("site {rva:#x} mismatch")); }
    }
    for (rva, old, v) in sites { unsafe { repoint(rva, [0xD9, 0x05], old, [0xD9, 0x05], v)?; } }
    unsafe {
        setf(0xD1792C, 105.0, draw)?;
        setf(0xD1773C, 25.0, shadow)?;
        setf(0xD17740, 25.0, shadow)?;
    }
    Ok(format!("LOD1 {l1}/{l1i} m, LOD2 {l2}/{l2i} m, draw {draw} m, shadows {shadow} m"))
}

/// ClipRange 3 far plane (slice 4) 1000 m -> configurable.
unsafe fn draw_distance() -> Result<String, String> {
    let d = ini::f32v("World", "DrawDistance", 2500.0);
    if d <= 1000.0 { return Err("vanilla (ini)".into()); }
    unsafe { repoint(0x242F8E, [0xD9, 0x05], 0xB7D630, [0xD9, 0x05], d.min(10000.0))?; }
    Ok(format!("far plane 1000 -> {d} m"))
}

/// Camera FOV setter: keep vertical FOV of a reference aspect (h/w) and widen horizontally on wider screens (Hor+).
unsafe fn hor_plus() -> Result<String, String> {
    if !ini::boolv("FOV", "HorPlus", true) { return Err("disabled in ini".into()); }
    let ref_w = ini::f32v("FOV", "ReferenceAspectWidth", 16.0);
    let ref_h = ini::f32v("FOV", "ReferenceAspectHeight", 9.0);
    let r = super::fixes::pool_f32(ref_h / ref_w);
    let mut n = vec![0xD9, 0x05]; n.extend_from_slice(&(r as u32).to_le_bytes());
    n.extend_from_slice(&hx("D874240CD9E8DBF1DAC1DDD9DEC9838E0401000003C60600D99610010000D84C240CD99E140100005EC20800"));
    assert_eq!(n.len(), 50);
    unsafe { swap(0xA0501A, "d95c2408d9442408838e0401000003d95c2408c60600d9442408d99610010000d84c240cd99e140100005ec20800cccccccc", &n)?; }
    Ok(format!("Hor+ relative to {ref_w}:{ref_h}"))
}

/// Main camera adds an unused global (always 0) to the FOV: turn it into a multiplier.
unsafe fn fov_mult() -> Result<String, String> {
    let m = ini::f32v("FOV", "FOVMultiplier", 1.1);
    if (m - 1.0).abs() < 0.001 { return Err("1.0 (ini)".into()); }
    let m = m.clamp(0.5, 1.6);
    let c = super::fixes::pool_f32(m);
    for rva in [0x3A4B1usize, 0x3A8C1] {
        let mut o = vec![0xD8, 0x05]; o.extend_from_slice(&((b() + 0xE1008C) as u32).to_le_bytes());
        if bytes_at(b() + rva, 6) != o { return Err(format!("site {rva:#x} mismatch")); }
        let mut n = vec![0xD8, 0x0D]; n.extend_from_slice(&(c as u32).to_le_bytes());
        unsafe { write(b() + rva, &n); }
    }
    Ok(format!("FOV x{m}"))
}

unsafe fn water() -> Result<String, String> {
    let s = ini::i32v("World", "WaterReflectionScale", 2);
    if s <= 1 { return Err("vanilla (ini)".into()); }
    let s = s.min(4) as f32;
    unsafe { setf(0xDC13D8, 512.0, 512.0 * s)?; setf(0xDC13DC, 128.0, 128.0 * s)?; }
    Ok(format!("water reflection {}x{}", 512.0 * s, 128.0 * s))
}

/// Camera-shake spring stepped a fixed 0.05 s per frame: rewrite integrator with s = dt*60 (identical at 60 FPS).
unsafe fn camera_shake() -> Result<String, String> {
    if !ini::boolv("Fixes", "CameraShakeFPSFix", true) { return Err("disabled in ini".into()); }
    let old = "8b45ecd9008b4decd86110d95df4d945f48b55ecd84a04d95df88b45ecd9400cd9e08b4decd84908d95df08b55ecd94210d95dfc8b45ecd9400cd845f8d845f0d84d088b4decd841108b55ecd95a108b45ecd94010d865fcd875088b4decd9590c";
    let dt = b() + 0x10E1C68;
    let k60 = b() + 0xB79920; // existing 1/60 constant
    let mut n = hx("8b4decd905"); n.extend_from_slice(&(dt as u32).to_le_bytes());
    n.extend_from_slice(&[0xD8, 0x35]); n.extend_from_slice(&(k60 as u32).to_le_bytes());
    n.extend_from_slice(&hx("d901d86110d84904d9410cd84908dee9d8c9d8410cd9510cd84d08dec9d84110d95910"));
    while n.len() < 97 { n.push(0x90); }
    unsafe { swap(0x27B76B, old, &n)?; }
    Ok("camera shake spring time-based".into())
}

/// Skid marks fade 0.01 alpha per frame -> per second-based (subtract dt).
unsafe fn skid_marks() -> Result<String, String> {
    if !ini::boolv("Fixes", "SkidMarkFPSFix", true) { return Err("disabled in ini".into()); }
    let mut n = vec![0xD8, 0x25]; n.extend_from_slice(&((b() + 0x10E1C68) as u32).to_le_bytes());
    unsafe { swap(0x20E9C7, "dc25b8eef900", &n)?; }
    Ok("skid mark fade time-based".into())
}
