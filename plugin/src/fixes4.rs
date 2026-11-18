//! Round-4 (review) fixes: high-FPS physics/camera caves.
//! s = dt*60 (60-FPS frames represented by this frame), always capped to 1 so nothing explodes on long frames.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}
fn hx(s: &str) -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0)).collect() }
fn b() -> usize { base() }
fn le(a: usize) -> [u8; 4] { (a as u32).to_le_bytes() }
const MIN1: &str = "d9e8dbf1dbc1ddd9"; // fld1; fcomi st1; fcmovnb st0,st1; fstp st1  => min(s,1), NaN->1

fn check(rva: usize, old: &str) -> Result<(), String> {
    let o = hx(old);
    let cur = bytes_at(b() + rva, o.len());
    if cur != o { Err(format!("rva {rva:#x}: {} != expected", hex(&cur))) } else { Ok(()) }
}

/// Build a cave: `body`, then jmp back to `ret_rva`; hook `site_rva` (len bytes) with a jmp to it.
unsafe fn cave(site_rva: usize, len: usize, body: Vec<u8>, ret_rva: usize) {
    let p = super::fixes::pool_ref();
    let at = p.put_bytes(&vec![0xCCu8; body.len() + 8]);
    let mut c = body;
    let here = at + c.len();
    c.push(0xE9); c.extend_from_slice(&(((b() + ret_rva) as isize - (here as isize + 5)) as i32).to_le_bytes());
    unsafe { write(at, &c); jmp(b() + site_rva, at, len); }
}

fn dt() -> usize { b() + 0x10E1C68 }
fn c60() -> usize { super::fixes::pool_f32(60.0) }
/// fld [dt]; fmul [60]; min(.,1)
fn s_capped() -> Vec<u8> { let mut v = vec![0xD9, 0x05]; v.extend(le(dt())); v.extend([0xD8, 0x0D]); v.extend(le(c60())); v.extend(hx(MIN1)); v }

static mut FRAME60: usize = 0;
static mut SNAP: usize = 0;
pub fn frame60_addr() -> usize { unsafe { FRAME60 } }
/// Per-frame c^(dt*60) values: [0.999, 0.99, 0.98, 0.93, 0.1, 0.9, 1-0.9^s, 1/(1-0.8^s)]
pub static mut POW: [usize; 13] = [0; 13]; // +[8]=s, [9]=0.001*s, [10]=1-0.5^s, [11]=unused, [12]=1-0.1^s

pub fn apply() {
    unsafe {
        run("CameraShakeFPS", camera_shake());
        run("ShakeKicksFPS", shake_kicks());
        run("HavokPushFPS", havok_push());
        run("Frame60Counter", frame60());
        run("SkidMarksFPS", skid_marks());
        run("GroundSnapFPS", ground_snap());
        run("DtCap", dt_cap());
    }
}

/// Camera-shake spring: time-based integrator with s capped at 1 (stable) + NaN/Inf self-heal.
unsafe fn camera_shake() -> Result<String, String> {
    if !ini::boolv("Fixes", "CameraShakeFPSFix", true) { return Err("disabled in ini".into()); }
    if b() != 0x400000 { return Err("unexpected image base".into()); }
    let old = "8b45ecd9008b4decd86110d95df4d945f48b55ecd84a04d95df88b45ecd9400cd9e08b4decd84908d95df08b55ecd94210d95dfc8b45ecd9400cd845f8d845f0d84d088b4decd841108b55ecd95a108b45ecd94010d865fcd875088b4decd9590c";
    check(0x27B76B, old)?;
    let new = hx("8b4decd905681c4e01d8352099fb00d9e8dbf1dbc1ddd9d901d86110d84904d9410cd84908dee9d8c9d8410cd9510cd84d08dec9d84110d959108b4110250000807f3d0000807f740f8b410c250000807f3d0000807f750833c089410c89411090");
    if new.len() != 97 { return Err(format!("internal length {}", new.len())); }
    unsafe { write(b() + 0x27B76B, &new); }
    Ok("shake spring time-based, stable, NaN-safe".into())
}

/// Per-frame random shake kicks scaled by sqrt(min(s,1)) (RMS matches 60 FPS).
unsafe fn shake_kicks() -> Result<String, String> {
    if !ini::boolv("Fixes", "CameraShakeFPSFix", true) { return Err("disabled in ini".into()); }
    check(0x2783C8, "d9e8dec9dec9d99de8fdffff")?;
    check(0x278523, "d88d50ffffff")?;
    let mut c1 = vec![0xDE, 0xC9]; c1.extend(s_capped()); c1.extend([0xD9, 0xFA, 0xDE, 0xC9]); c1.extend(hx("d99de8fdffff"));
    let mut c2 = hx("d88d50ffffff"); c2.extend(s_capped()); c2.extend([0xD9, 0xFA, 0xDE, 0xC9]);
    unsafe { cave(0x2783C8, 12, c1, 0x2783D4); cave(0x278523, 6, c2, 0x278529); }
    Ok("kicks x sqrt(s)".into())
}

/// hkpCharacterProxy::applySurfaceInteractions: impulse -= projVel*mass every step -> scale projVel by min(stepDt*60,1).
unsafe fn havok_push() -> Result<String, String> {
    if !ini::boolv("Fixes", "HavokPushFix", true) { return Err("disabled in ini".into()); }
    check(0x744D8E, "d86c2418d95c2418")?; // fsubr [esp+18]; fstp [esp+18]
    check(0x744D96, "eb18")?;                                       // jmp 0xb44db0
    let mut c = hx("8b4508d94008");                                 // mov eax,[ebp+8]; fld [eax+8] (Havok step dt)
    c.extend([0xD8, 0x0D]); c.extend(le(c60())); c.extend(hx(MIN1)); // *60, min 1
    c.extend(hx("dec9d86c2418d95c2418"));                           // fmulp; fsubr [esp+18]; fstp [esp+18]
    unsafe { cave(0x744D8E, 8, c, 0x744DB0); }
    Ok("character proxy momentum transfer scaled by step".into())
}

/// Virtual 60 Hz frame counter (accumulates dt*60) next to the engine's frame counter.
unsafe fn frame60() -> Result<String, String> {
    if !ini::boolv("Fixes", "FrameTimers60Hz", true) { return Err("disabled in ini".into()); }
    check(0x3A423, "013d60002101")?;
    let p = super::fixes::pool_ref();
    let acc = p.put_f32(0.0); let tmp = p.put_f32(0.0); let g = p.put_f32(0.0);
    unsafe { *(g as *mut u32) = 0; FRAME60 = g; }
    let mut c = hx("013d60002101");
    c.extend([0xD9, 0x05]); c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(c60()));
    c.extend([0xD8, 0x05]); c.extend(le(acc));                 // fadd [acc]
    c.extend(hx("d9c0d9fcdce9"));                              // fld st0; frndint; fsub st1,st0
    c.extend([0xDB, 0x1D]); c.extend(le(tmp));                 // fistp [tmp]
    c.extend([0xD9, 0x1D]); c.extend(le(acc));                 // fstp [acc]
    c.extend([0x50, 0xA1]); c.extend(le(tmp));                 // push eax; mov eax,[tmp]
    c.extend([0x01, 0x05]); c.extend(le(g)); c.push(0x58);     // add [g],eax; pop eax
    // per-frame ground-snap factor 1 - (1-f)^(dt*60), computed here where the FPU stack is empty
    let f = unsafe { read::<f32>(b() + 0xD1B6AC) };
    if ini::boolv("Fixes", "GroundSnapFPSFix", true) && f > 0.0 && f < 1.0 {
        let snap = p.put_f32(f);
        let l60 = p.put_f32(60.0 * (1.0 - f).log2());
        c.extend([0xD9, 0x05]); c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(l60));
        c.extend(hx("d9c0d9fcdce9d9c9d9f0d9e8dec1d9fdddd9d9e8dee1"));
        c.extend([0xD9, 0x1D]); c.extend(le(snap));            // fstp [snap]
        unsafe { SNAP = snap; }
    }
    // per-frame decay factors for per-frame lerps elsewhere (fixes7): c^(dt*60)
    let pow_seq = hx("d9c0d9fcdce9d9c9d9f0d9e8dec1d9fdddd9");
    let bases = [0.999f32, 0.99, 0.98, 0.93, 0.1, 0.9, 0.8];
    let mut outs = [0usize; 7];
    for (i, &cb) in bases.iter().enumerate() {
        let out = p.put_f32(cb); let l = p.put_f32(60.0 * cb.log2()); outs[i] = out;
        c.extend([0xD9, 0x05]); c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(l));
        c.extend(pow_seq.iter()); c.extend([0xD9, 0x1D]); c.extend(le(out));
    }
    let om = p.put_f32(0.1); let inv = p.put_f32(5.0);
    c.extend([0xD9, 0xE8, 0xD8, 0x25]); c.extend(le(outs[5])); c.extend([0xD9, 0x1D]); c.extend(le(om));   // 1-0.9^s
    c.extend([0xD9, 0xE8, 0xD8, 0x25]); c.extend(le(outs[6])); c.extend([0xD9, 0xE8, 0xDE, 0xF1, 0xD9, 0x1D]); c.extend(le(inv)); // 1/(1-0.8^s)
    // s and 0.001*s; 1-0.5^s; 1-0.1^s
    let sv = p.put_f32(1.0); let kv = p.put_f32(0.001); let c001 = p.put_f32(0.001);
    c.extend([0xD9, 0x05]); c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(c60()));
    c.extend([0xD9, 0x15]); c.extend(le(sv)); c.extend([0xD8, 0x0D]); c.extend(le(c001)); c.extend([0xD9, 0x1D]); c.extend(le(kv));
    let p05 = p.put_f32(0.5); let l05 = p.put_f32(-60.0); let om05 = p.put_f32(0.5);
    c.extend([0xD9, 0x05]); c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(l05));
    c.extend(pow_seq.iter()); c.extend([0xD9, 0x1D]); c.extend(le(p05));
    c.extend([0xD9, 0xE8, 0xD8, 0x25]); c.extend(le(p05)); c.extend([0xD9, 0x1D]); c.extend(le(om05));
    let om01 = p.put_f32(0.9);
    c.extend([0xD9, 0xE8, 0xD8, 0x25]); c.extend(le(outs[4])); c.extend([0xD9, 0x1D]); c.extend(le(om01));
    unsafe { POW = [outs[0], outs[1], outs[2], outs[3], outs[4], outs[5], om, inv, sv, kv, om05, 0, om01]; }
    unsafe { cave(0x3A423, 6, c, 0x3A429); }
    Ok(format!("g_frame60 at {g:#x}"))
}

/// Skid marks: fade 0.6/s (vanilla 0.01/frame at 60 FPS) and age in 60 Hz frames.
unsafe fn skid_marks() -> Result<String, String> {
    if !ini::boolv("Fixes", "SkidMarkFPSFix", true) { return Err("disabled in ini".into()); }
    check(0x20E9C7, "dc25b8eef900")?;
    let c06 = super::fixes::pool_f32(0.6);
    let mut c = vec![0xD9, 0x05]; c.extend(le(dt())); c.extend([0xD8, 0x0D]); c.extend(le(c06)); c.extend([0xDE, 0xE9]);
    unsafe { cave(0x20E9C7, 6, c, 0x20E9CD); }
    let mut ages = "";
    let g = unsafe { FRAME60 };
    if g != 0 && check(0x20E77E, "8b1d60002101").is_ok() && check(0x20E993, "8b1d60002101").is_ok() {
        let mut n = vec![0x8B, 0x1D]; n.extend(le(g));
        unsafe { write(b() + 0x20E77E, &n); write(b() + 0x20E993, &n); }
        ages = " + 60 Hz ages";
    }
    Ok(format!("fade 0.6/s{ages}"))
}

/// Ground snap y += (ground-y)*0.4 per frame -> 1-0.6^(dt*60).
unsafe fn ground_snap() -> Result<String, String> {
    if !ini::boolv("Fixes", "GroundSnapFPSFix", true) { return Err("disabled in ini".into()); }
    let f = unsafe { read::<f32>(b() + 0xD1B6AC) };
    let mut o = vec![0xD8, 0x0D]; o.extend(le(b() + 0xD1B6AC));
    if bytes_at(b() + 0x184AF9, 6) != o { return Err("site mismatch".into()); }
    if !(f > 0.0 && f < 1.0) { return Err(format!("unexpected factor {f}")); }
    let snap = unsafe { SNAP };
    if snap == 0 { return Err("needs FrameTimers60Hz=1 (factor is computed in the main loop)".into()); }
    let mut n = vec![0xD8, 0x0D]; n.extend(le(snap));             // fmul [snap]
    unsafe { write(b() + 0x184AF9, &n); }
    Ok(format!("snap factor 1-{:.2}^(dt*60)", 1.0 - f))
}

/// FrameClock max dt outside cinematics 0.2 s -> 0.1 s (repoint the two compares, constants are shared).
unsafe fn dt_cap() -> Result<String, String> {
    let cap = ini::f32v("Fixes", "MaxFrameTime", 0.1);
    if cap >= 0.2 || cap <= 0.0 { return Err("vanilla (ini)".into()); }
    check(0x9B6FB6, "dc1d5826f800")?;
    check(0x9B6FC9, "d905acd3f700")?;
    let p = super::fixes::pool_ref();
    let d = p.put_bytes(&(cap as f64).to_le_bytes());
    let f = p.put_f32(cap);
    let mut a = vec![0xDC, 0x1D]; a.extend(le(d));
    let mut c = vec![0xD9, 0x05]; c.extend(le(f));
    unsafe { write(b() + 0x9B6FB6, &a); write(b() + 0x9B6FC9, &c); }
    Ok(format!("max dt {cap} s"))
}
