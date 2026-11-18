//! Round-5 fixes: instant pause menu (keep streamed blocks resident), streaming threads affinity, impact-FX throttle in 60 Hz frames.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}
fn hx(s: &str) -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0)).collect() }
fn b() -> usize { base() }
fn check(rva: usize, old: &str) -> Result<(), String> {
    let o = hx(old);
    let cur = bytes_at(b() + rva, o.len());
    if cur != o { Err(format!("rva {rva:#x}: {} != expected", hex(&cur))) } else { Ok(()) }
}

pub fn apply() {
    unsafe {
        run("KeepPauseMenuLoaded", keep_blocks("KeepPauseMenuLoaded", &[(0x5BEEB2, "e8191e0300"), (0x3A4743, "e888c52400"), (0x3A23F9, "e8d2e82400")]));
        run("KeepWorldMapLoaded", keep_blocks("KeepWorldMapLoaded", &[(0x3A4723, "e8a8c52400"), (0x3A23DE, "e8ede82400")]));
        run("StreamingThreads", streaming_threads());
        run("ImpactFXThrottle", impact_fx());
    }
}

/// Replace "call FUN_009f0cd0" (queue block unload; callee pops its 1 arg) with "add esp,4" so the block stays loaded.
unsafe fn keep_blocks(key: &str, sites: &[(usize, &str)]) -> Result<String, String> {
    if !ini::boolv("UI", key, true) { return Err("disabled in ini".into()); }
    for &(rva, old) in sites { check(rva, old)?; }
    for &(rva, _) in sites { unsafe { write(b() + rva, &[0x83, 0xC4, 0x04, 0x90, 0x90]); } }
    Ok(format!("{} unload sites removed", sites.len()))
}

/// Engine threads 5-7 (job worker, stream read, stream process/decompress) were all pinned to the last logical CPU.
unsafe fn streaming_threads() -> Result<String, String> {
    if !ini::boolv("Fixes", "SpreadStreamingThreads", true) { return Err("disabled in ini".into()); }
    check(0x587574, "8b4c24148b7cb42883c1ffba01000000d3e2")?;
    unsafe { write(b() + 0x587574, &hx("8b4c24148b7cb42833d2424249d3e24a4a90")); }
    Ok("stream/worker threads 5-7 use CPUs 1..n-1 (vanilla: last CPU only)".into())
}

/// Impact-effect throttle (5-slot ring, skip StartFX for 15 frames) -> 60 Hz frames.
unsafe fn impact_fx() -> Result<String, String> {
    if !ini::boolv("Fixes", "FrameTimers60Hz", true) { return Err("disabled in ini".into()); }
    let g = super::fixes4::frame60_addr();
    if g == 0 { return Err("needs the 60 Hz frame counter".into()); }
    check(0x52709C, "a160002101")?;
    check(0x527142, "8b0d60002101")?;
    let mut r = vec![0xA1]; r.extend((g as u32).to_le_bytes());
    let mut w = vec![0x8B, 0x0D]; w.extend((g as u32).to_le_bytes());
    unsafe { write(b() + 0x52709C, &r); write(b() + 0x527142, &w); }
    Ok("impact FX throttle in 60 Hz frames".into())
}
