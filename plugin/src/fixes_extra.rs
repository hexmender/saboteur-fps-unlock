//! Additional fixes (round 3).
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}

pub fn apply_extra() {
    unsafe {
        run("ShadowMapSize", shadow_size());
        run("ShadowCascades", shadow_cascades());
        run("SmallPropShadows", small_prop_shadows());
        run("RawMouse", raw_mouse());
        run("UISpeed", ui_speed());
        run("LegalScreen", legal_screen());
    }
}

/// Little x86 emitter for code caves.
struct Asm(Vec<u8>);
impl Asm {
    fn b(&mut self, x: &[u8]) -> &mut Self { self.0.extend_from_slice(x); self }
    fn op_abs(&mut self, op: &[u8], addr: usize) -> &mut Self { self.b(op); self.0.extend_from_slice(&(addr as u32).to_le_bytes()); self }
    fn jmp_to(&mut self, cave_at: usize, to: usize) -> &mut Self {
        let here = cave_at + self.0.len();
        self.b(&[0xE9]); self.0.extend_from_slice(&((to as isize - (here as isize + 5)) as i32).to_le_bytes()); self
    }
}

/// Mouse look was frame-rate dependent: the per-frame mouse delta is later multiplied by dt again,
/// so at 240 FPS the mouse turned 4x slower than at 60. Divide the mouse scale (0.04) by (60*dt) at the source,
/// so angle per mouse count is constant at any FPS (calibrated to feel like 60 FPS). Gamepad is untouched.
unsafe fn raw_mouse() -> Result<String, String> {
    if !ini::boolv("Input", "FrameIndependentMouse", true) { return Err("disabled in ini".into()); }
    let site = base() + 0x286AAC;
    let k004 = base() + 0xBCBF08;
    let mut exp = vec![0xDD, 0x05]; exp.extend_from_slice(&(k004 as u32).to_le_bytes());
    if bytes_at(site, 6) != exp { return Err(format!("site mismatch: {}", hex(&bytes_at(site, 6)))); }
    let raw_dt = base() + 0x10E1C88; // FrameClock raw elapsed seconds
    let mult = ini::f32v("Input", "MouseSensitivityMultiplier", 1.0);
    let p = super::fixes::pool_ref();
    let cmin = p.put_f32(0.0005);
    let c60 = p.put_f32(60.0 / mult.max(0.01));
    let cave = p.put_bytes(&[0xCC; 64]);
    let mut a = Asm(Vec::new());
    a.b(&[0x50])                       // push eax
     .op_abs(&[0xD9, 0x05], raw_dt)    // fld dword [raw_dt]
     .op_abs(&[0xD8, 0x15], cmin)      // fcom dword [cmin]
     .b(&[0xDF, 0xE0, 0xF6, 0xC4, 0x01, 0x74, 0x08]) // fnstsw ax; test ah,1; jz +8
     .b(&[0xDD, 0xD8])                 // fstp st0
     .op_abs(&[0xD9, 0x05], cmin)      // fld dword [cmin]
     .op_abs(&[0xD8, 0x0D], c60)       // fmul dword [60/mult]
     .op_abs(&[0xDC, 0x3D], k004)      // fdivr qword [0.04]  -> 0.04 / (60*dt)
     .b(&[0x58]);                      // pop eax
    a.jmp_to(cave, site + 6);
    unsafe { write(cave, &a.0); jmp(site, cave, 6); }
    Ok(format!("mouse turn independent of FPS (x{mult})"))
}

/// Scaleform UI (pause menu, fades, HUD widgets) advances with real dt: scale it.
unsafe fn ui_speed() -> Result<String, String> {
    let m = ini::f32v("UI", "AnimationSpeed", 2.0);
    if m <= 1.0 { return Err("vanilla (ini)".into()); }
    let site = base() + 0x39E5AB;
    if bytes_at(site, 15) != [0xD9, 0x44, 0x24, 0x14, 0x8B, 0x42, 0x78, 0x6A, 0x02, 0x51, 0xD9, 0x1C, 0x24, 0xFF, 0xD0] {
        return Err(format!("site mismatch: {}", hex(&bytes_at(site, 15))));
    }
    let p = super::fixes::pool_ref();
    let cm = p.put_f32(m.min(8.0));
    let cave = p.put_bytes(&[0xCC; 32]);
    let mut a = Asm(Vec::new());
    a.b(&[0xD9, 0x44, 0x24, 0x14]).op_abs(&[0xD8, 0x0D], cm).b(&[0x8B, 0x42, 0x78]);
    a.jmp_to(cave, site + 7);
    unsafe { write(cave, &a.0); jmp(site, cave, 7); }
    Ok(format!("UI animations x{m}"))
}

/// Legal screen: state 4 waits 4.5 s of real time.
unsafe fn legal_screen() -> Result<String, String> {
    let s = ini::f32v("UI", "LegalScreenSeconds", 0.0);
    if s >= 4.5 { return Err("vanilla (ini)".into()); }
    expect_f32(&[(0xBF58A8, 4.5)])?;
    unsafe { wr_f32(0xBF58A8, s.max(0.0)); }
    Ok(format!("legal screen {s} s"))
}

fn rd_f32(rva: usize) -> f32 { unsafe { read::<f32>(base() + rva) } }
unsafe fn wr_f32(rva: usize, v: f32) { unsafe { write(base() + rva, &v.to_le_bytes()) } }

fn expect_f32(list: &[(usize, f32)]) -> Result<(), String> {
    for &(rva, v) in list { let c = rd_f32(rva); if c != v { return Err(format!("rva {rva:#x} = {c}, expected {v} (different build?)")); } }
    Ok(())
}

/// 5 shadow depth maps (one per depth slice), 1024^2 in vanilla. Width/height floats + texel-snap constant.
unsafe fn shadow_size() -> Result<String, String> {
    let n = ini::i32v("Shadows", "ShadowMapSize", 2048);
    if n <= 0 || n == 1024 { return Err("vanilla 1024 (ini)".into()); }
    let n = n.clamp(512, 8192) as f32;
    expect_f32(&[(0xDD61B4, 1024.0), (0xDD61B8, 1024.0), (0xD20EE4, 1024.0)])?;
    unsafe { wr_f32(0xDD61B4, n); wr_f32(0xDD61B8, n); wr_f32(0xD20EE4, n); }
    Ok(format!("{n}x{n}"))
}

/// Depth-slice set 3 (SliceQuality 3): vanilla 0.1-1.5 / 1.5-5 / 5-20 / 20-100 / 80-far.
/// Shadow maps are fitted per slice and switch hard at the borders, so push the borders out.
unsafe fn shadow_cascades() -> Result<String, String> {
    if !ini::boolv("Shadows", "RebalanceCascades", true) { return Err("disabled in ini".into()); }
    let s0 = ini::f32v("Shadows", "Cascade0Far", 4.0);
    let s1 = ini::f32v("Shadows", "Cascade1Far", 15.0);
    let s2 = ini::f32v("Shadows", "Cascade2Far", 40.0);
    let s3 = ini::f32v("Shadows", "Cascade3Far", 150.0);
    // set 3 base rva 0xD20B8C; slice k at +k*12 {near, far, flags}
    let b = 0xD20B8C;
    expect_f32(&[(b + 4, 1.5), (b + 12, 1.5), (b + 16, 5.0), (b + 24, 5.0), (b + 28, 20.0), (b + 36, 20.0), (b + 40, 100.0)])?;
    unsafe {
        wr_f32(b + 4, s0); wr_f32(b + 12, s0);
        wr_f32(b + 16, s1); wr_f32(b + 24, s1);
        wr_f32(b + 28, s2); wr_f32(b + 36, s2);
        wr_f32(b + 40, s3);
    }
    // the minimap capture restores slice 3 far to a constant 100.0 (fld [0xf7d640] at 0x43b83c): point it at our value
    let site = base() + 0x3B83C;
    let cur = bytes_at(site, 6);
    if cur[0] == 0xD9 && cur[1] == 0x05 && unsafe { read::<u32>(site + 2) } as usize == base() + 0xB7D640 {
        let c = super::fixes::pool_f32(s3);
        let mut v = vec![0xD9, 0x05]; v.extend_from_slice(&(c as u32).to_le_bytes());
        unsafe { write(site, &v); }
    } else { logf!("       (minimap slice-3 restore site not matched: {})", hex(&cur)); }
    Ok(format!("{s0} / {s1} / {s2} / {s3} m"))
}

/// Fallback shadow mask for models not in ModelInfo.txt: vanilla casts small props into maps 0-1 or 0-2 only.
unsafe fn small_prop_shadows() -> Result<String, String> {
    let n = ini::i32v("Shadows", "SmallPropShadowCascades", 4);
    if !(4..=5).contains(&n) { return Err("vanilla (ini)".into()); }
    let p = base() + 0x23965D;
    let ok = bytes_at(p, 2) == [0x8B, 0xDE] && bytes_at(p + 9, 3) == [0x8D, 0x5E, 0xFC] && bytes_at(p + 19, 3) == [0x8D, 0x5E, 0xF8];
    if !ok { return Err(format!("bytes mismatch: {}", hex(&bytes_at(p, 22)))); }
    unsafe {
        if n == 5 { write(p, &[0x90, 0x90]); write(p + 9, &[0x90, 0x90, 0x90]); write(p + 19, &[0x90, 0x90, 0x90]); }
        else { write(p, &[0xB3, 0x0F]); write(p + 9, &[0x8D, 0x5E, 0x08]); write(p + 19, &[0x8D, 0x5E, 0x00]); }
    }
    Ok(format!("small props cast into maps 0-{}", n - 1))
}
