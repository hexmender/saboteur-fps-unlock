//! Round-6 fixes: traffic density, AA edge-detect resolution, (experimental) distant halos.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}
fn b() -> usize { base() }

pub fn apply() {
    unsafe {
        run("TrafficDensity", traffic());

        run("DistantHalos", halos());
    }
}

/// Road segment density = 0.5*activity*length*0.03125 (cars per metre). Repoint both loads to 0.03125*mult.
unsafe fn traffic() -> Result<String, String> {
    let m = ini::f32v("World", "TrafficDensity", 1.5);
    if (m - 1.0).abs() < 0.01 { return Err("1.0 (ini)".into()); }
    let m = m.clamp(0.25, 3.0) as f64;
    let mut old = vec![0xDD, 0x05]; old.extend(((b() + 0xBAD120) as u32).to_le_bytes());
    for rva in [0x504D16usize, 0x504FEF] { if bytes_at(b() + rva, 6) != old { return Err(format!("site {rva:#x} mismatch")); } }
    let c = super::fixes::pool_ref().put_bytes(&(0.03125f64 * m).to_le_bytes());
    let mut n = vec![0xDD, 0x05]; n.extend((c as u32).to_le_bytes());
    for rva in [0x504D16usize, 0x504FEF] { unsafe { write(b() + rva, &n); } }
    Ok(format!("traffic x{m} (hard cap 64 cars)"))
}

fn config_resolution() -> Option<(f32, f32)> {
    let prof = std::env::var("USERPROFILE").ok()?;
    let mg = std::path::Path::new(&prof).join("Documents").join("My Games");
    for e in std::fs::read_dir(&mg).ok()? {
        let e = e.ok()?;
        if !e.file_name().to_string_lossy().starts_with("The Saboteur") { continue; }
        let s = std::fs::read_to_string(e.path().join("SaveGames").join("config.ini")).ok()?;
        let mut w = None; let mut h = None;
        for l in s.lines() {
            let t: Vec<&str> = l.split_whitespace().collect();
            if t.len() >= 2 && t[0] == "ScreenWidth" { w = t[1].parse::<f32>().ok(); }
            if t.len() >= 2 && t[0] == "ScreenHeight" { h = t[1].parse::<f32>().ok(); }
        }
        if let (Some(w), Some(h)) = (w, h) { return Some((w, h)); }
    }
    None
}

/// FSAAEdgeBlur shader samples neighbours with offsets hard-coded for 1280x720 (def c2): use the real resolution.
unsafe fn aa_edge() -> Result<String, String> {
    if !ini::boolv("Graphics", "FixAAEdgeResolution", true) { return Err("disabled in ini".into()); }
    let p = b() + 0xD90BC8;
    if bytes_at(p, 8) != [0x61, 0x0B, 0xB6, 0xBA, 0xCD, 0xCC, 0x4C, 0x3A] { return Err("shader constant mismatch".into()); }
    let (w, h) = config_resolution().ok_or("config.ini resolution not found")?;
    if w < 640.0 || h < 480.0 { return Err(format!("odd resolution {w}x{h}")); }
    unsafe { write(p, &(-1.0f32 / h).to_le_bytes()); write(p + 4, &(1.0f32 / w).to_le_bytes()); }
    Ok(format!("edge-detect offsets for {w}x{h} (was 1280x720)"))
}

/// Light halos with 0 occlusion-query samples (sub-pixel / far away) were treated as invisible.
unsafe fn halos() -> Result<String, String> {
    if !ini::boolv("Graphics", "DistantHalos", false) { return Err("disabled in ini (experimental)".into()); }
    let p = b() + 0x3DC826;
    if bytes_at(p, 2) != [0xD9, 0xEE] { return Err("site mismatch".into()); }
    unsafe { write(p, &[0xD9, 0xE8]); }
    Ok("sub-pixel halos stay visible".into())
}
