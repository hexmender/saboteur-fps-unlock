//! Round-8 fixes: per-frame decays without dt (trains, falling leaves, vehicle camera sway) and vehicle speed-FOV ramps.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}
fn hx(s: &str) -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0)).collect() }
fn b() -> usize { base() }

/// Repoint (rva, old bytes) to a 2-byte opcode + abs32 of a per-frame value.
unsafe fn repoint(list: &[(usize, &str, [u8; 2], usize)]) -> Result<(), String> {
    for &(rva, old, _, v) in list {
        if v == 0 { return Err("needs FrameTimers60Hz=1 (values computed in the main loop)".into()); }
        let o = hx(old);
        if bytes_at(b() + rva, o.len()) != o { return Err(format!("site {rva:#x} mismatch")); }
    }
    for &(rva, _, op, v) in list { let mut n = op.to_vec(); n.extend((v as u32).to_le_bytes()); unsafe { write(b() + rva, &n); } }
    Ok(())
}

pub fn apply() {
    unsafe {
        let p = *core::ptr::addr_of!(super::fixes4::POW);
        run("TrainDragFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            repoint(&[(0x1FBEB9, "dc0d186ef800", [0xD8, 0x0D], p[0]), (0x1FBEC5, "dc0df06df800", [0xD8, 0x0D], p[1])]).map(|_| "train speed decay 0.999/0.99 per 60 Hz frame".into())
        } else { Err("disabled in ini".into()) });
        run("FallingLeavesFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            repoint(&[(0x2CF7C0, "d90584411201", [0xD9, 0x05], p[2]), (0x2CFC8E, "d80d6c411201", [0xD8, 0x0D], p[3])]).map(|_| "leaf drag/spin per 60 Hz frame".into())
        } else { Err("disabled in ini".into()) });
        run("VehicleCamSwayFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            repoint(&[(0x27A04C, "d90544d5f700", [0xD9, 0x05], p[4]), (0x27A288, "dc0d3826f800", [0xD8, 0x0D], p[5]),
                      (0x27A28E, "dc0548d5f700", [0xD8, 0x05], p[6]), (0x27A389, "d90548b1f700", [0xD9, 0x05], p[7])]).map(|_| "vehicle camera sway filters time-based".into())
        } else { Err("disabled in ini".into()) });
        run("LeafSpinFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            // spin input k*s, applied angle spin*s (store unscaled), normal drag 1-0.5^s
            let mut a = vec![0xD9, 0x56, 0x60, 0xD8, 0x0D]; a.extend((p[8] as u32).to_le_bytes()); a.extend([0xD9, 0x1C, 0x24, 0x90, 0x90]);
            let site = b() + 0x2CFC94;
            if p[8] == 0 || bytes_at(site, 14) != hx("d95c2410d9442410d95660d91c24") { Err("site mismatch or no POW".into()) }
            else { repoint(&[(0x2CFC4A, "d80d68411201", [0xD8, 0x0D], p[9]), (0x2CF780, "d90564411201", [0xD9, 0x05], p[10])])
                   .map(|_| { unsafe { write(site, &a); } "leaf spin input/angle/normal drag time-based".into() }) }
        } else { Err("disabled in ini".into()) });
        run("RaceAILaneFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            repoint(&[(0x4FB3D5, "dc0d48d5f700", [0xD8, 0x0D], p[6]), (0x4FB3DD, "dc0d3826f800", [0xD8, 0x0D], p[5])]).map(|_| "race AI lane offset lerp time-based".into())
        } else { Err("disabled in ini".into()) });
        run("MoveInputSmoothingFPS", if ini::boolv("Fixes", "PerFrameDecayFix", true) {
            repoint(&[(0x1BF630, "d905f4c31101", [0xD9, 0x05], p[12])]).map(|_| "on-foot KB/mouse move-input smoothing time-based".into())
        } else { Err("disabled in ini".into()) });
        run("TurnFollow", if ini::boolv("Movement", "TurnFollow", true) {
            // facing = lerp(facing, run direction, min(speed,1) * 0.5) per frame -> 1-0.5^(dt*60)
            repoint(&[(0x162F77, "d80dc4a11101", [0xD8, 0x0D], p[10])]).map(|_| "body turns to the run direction at the 60 FPS rate".into())
        } else { Err("disabled in ini".into()) });
        run("SteeringSpeed", steering());
        run("VehicleSpeedFOV", speed_fov());
        run("MouseVertical", mouse_vertical());
    }
}

/// Player run-direction steering (FUN_0055bc70): turn rate starts at 0.3*speed, grows 12*speed/s, capped by
/// min(0.9*speed, 8). Scale all of them so W->A->S->D direction changes are quick.
unsafe fn steering() -> Result<String, String> {
    let m = ini::f32v("Movement", "SteeringSpeed", 3.0).clamp(0.25, 10.0);
    if (m - 1.0).abs() < 0.001 { return Err("1.0 (vanilla)".into()); }
    let k: [(usize, f32); 4] = [(0xD1A09C, 12.0), (0xD1A0A0, 0.9), (0xD1A0A4, 8.0), (0xD1A0A8, 0.3)];
    for (rva, v) in k { if unsafe { read::<f32>(b() + rva) } != v { return Err(format!("unexpected value at rva {rva:#x}")); } }
    for (rva, v) in k { unsafe { write(b() + rva, &(v * m).to_le_bytes()); } }
    Ok(format!("run direction steering x{m}"))
}

/// Speed-FOV blend ratio ramps 0.08/s up and 0.1/s down (very slow): make it snappier.
unsafe fn speed_fov() -> Result<String, String> {
    let up = ini::f32v("FOV", "VehicleSpeedFOVRampUp", 0.25);
    let down = ini::f32v("FOV", "VehicleSpeedFOVRampDown", 0.3);
    let a = b() + 0xD1D54C;
    let (cu, cd) = unsafe { (read::<f32>(a), read::<f32>(a + 4)) };
    if (cu - 0.08).abs() > 1e-6 || (cd - 0.1).abs() > 1e-6 { return Err(format!("unexpected values {cu} {cd}")); }
    unsafe { write(a, &up.to_le_bytes()); write(a + 4, &down.to_le_bytes()); }
    Ok(format!("speed FOV ramp {up}/s up, {down}/s down (vanilla 0.08/0.1)"))
}


/// Mouse Y sensitivity factor (esi+0x230 = (slider+0.25)/1.25) times MouseVerticalMultiplier.
unsafe fn mouse_vertical() -> Result<String, String> {
    let m = ini::f32v("Input", "MouseVerticalMultiplier", 1.0);
    if (m - 1.0).abs() < 0.001 { return Err("1.0 (ini)".into()); }
    let site = b() + 0x286A9E;
    if bytes_at(site, 8) != hx("def1d99e30020000") { return Err("site mismatch".into()); }
    let pool = super::fixes::pool_ref();
    let c = pool.put_f32(m.clamp(0.25, 4.0));
    let at = pool.put_bytes(&[0xCC; 32]);
    let mut v = hx("def1d80d"); v.extend((c as u32).to_le_bytes()); v.extend(hx("d99e30020000"));
    let here = at + v.len();
    v.push(0xE9); v.extend((((b() + 0x286AA6) as isize - (here as isize + 5)) as i32).to_le_bytes());
    unsafe { write(at, &v); jmp(site, at, 8); }
    Ok(format!("mouse vertical x{m}"))
}