//! Round-10 fixes (pop-in): grass / detail-object distance, pedestrian radius, traffic activation radius,
//! world streaming keep distances. Sites from static RE; rva = VA - 0x400000.
use crate::{ini, logf, mem};
use mem::*;

fn run(name: &str, r: Result<String, String>) {
    match r { Ok(m) => logf!("[ok]   {name}: {m}"), Err(e) => logf!("[skip] {name}: {e}") }
}
fn b() -> usize { base() }
fn le(a: usize) -> [u8; 4] { (a as u32).to_le_bytes() }
/// 2-byte x87 opcode + absolute address of rva `abs_rva` (D9 05 = fld m32, D8 1D = fcomp m32, D8 0D = fmul m32).
fn ins(op: [u8; 2], abs_rva: usize) -> Vec<u8> { let mut v = op.to_vec(); v.extend(le(b() + abs_rva)); v }
fn rd_f32(rva: usize) -> f32 { unsafe { read::<f32>(b() + rva) } }

pub fn apply() {
    unsafe {
        run("GrassDistance", grass());
        run("DetailObjectDistance", detail_objects());
        run("PedestrianRadius", peds());
        run("TrafficRadius", traffic_radius());
        run("StreamingDistance", streaming());
    }
}

/// WSDetailSystem default grass distance (+0x218) = fld [50.0]; the constant has 56 readers, so repoint this one load.
unsafe fn grass() -> Result<String, String> {
    let d = ini::f32v("World", "GrassDistance", 120.0);
    if (d - 50.0).abs() < 0.5 { return Err("vanilla 50 m (ini)".into()); }
    let d = d.clamp(20.0, 300.0);
    let site = b() + 0x3ECD20;
    if bytes_at(site, 6) != ins([0xD9, 0x05], 0xB7D3A8) { return Err(format!("site mismatch: {}", hex(&bytes_at(site, 6)))); }
    let mut n = vec![0xD9, 0x05]; n.extend(le(super::fixes::pool_f32(d)));
    unsafe { write(site, &n); }
    Ok(format!("grass draw distance 50 -> {d} m"))
}

/// Placed detail objects (FO_City_Grass_* ...): t = dist - fadeStart. The cave scales dist by 1/k first,
/// so both per-type fade distances (data) move k times farther with the same falloff.
unsafe fn detail_objects() -> Result<String, String> {
    let k = ini::f32v("World", "DetailObjectDistanceScale", 2.0);
    if (k - 1.0).abs() < 0.01 { return Err("1.0 (ini)".into()); }
    let k = k.clamp(1.0, 4.0);
    let site = b() + 0x3EBCF2;
    let orig = [0xD8u8, 0x67, 0x30, 0x83, 0xC4, 0x14]; // fsub [edi+0x30]; add esp,0x14
    if bytes_at(site, 6) != orig { return Err(format!("site mismatch: {}", hex(&bytes_at(site, 6)))); }
    let inv = super::fixes::pool_f32(1.0 / k);
    let mut body = vec![0xD8, 0x0D]; body.extend(le(inv)); // fmul [1/k]
    body.extend(orig);
    let at = super::fixes::pool_ref().put_bytes(&vec![0xCCu8; body.len() + 8]);
    let here = at + body.len();
    let rel = (b() + 0x3EBCF8) as i64 - (here as i64 + 5);
    body.push(0xE9); body.extend((rel as i32).to_le_bytes());
    unsafe { write(at, &body); jmp(site, at, 6); }
    Ok(format!("placed detail objects fade x{k} farther"))
}

/// WSAIDisposableHumanManager radii (.data, readers only inside the ped manager): despawn 115 m, no-visible-spawn 80 m.
unsafe fn peds() -> Result<String, String> {
    let outer = ini::f32v("World", "PedestrianRadius", 115.0);
    if (outer - 115.0).abs() < 0.5 { return Err("vanilla 115 m (ini)".into()); }
    let outer = outer.clamp(80.0, 300.0);
    let inner = ini::f32v("World", "PedestrianSpawnRadius", outer * 0.7).clamp(40.0, outer - 20.0);
    if rd_f32(0xDB75F4) != 115.0 || rd_f32(0xDB75F8) != 80.0 { return Err("values mismatch".into()); }
    unsafe { write(b() + 0xDB75F4, &outer.to_le_bytes()); write(b() + 0xDB75F8, &inner.to_le_bytes()); }
    Ok(format!("pedestrians exist to {outer} m, spawn unseen beyond {inner} m (vanilla 115/80)"))
}

/// Road nodes become active (spawn traffic) within 300 m (distSq < 90000; constant shared) - repoint the two compares.
unsafe fn traffic_radius() -> Result<String, String> {
    let r = ini::f32v("World", "TrafficRadius", 300.0);
    if (r - 300.0).abs() < 0.5 { return Err("vanilla 300 m (ini)".into()); }
    let r = r.clamp(150.0, 500.0);
    let old = ins([0xD8, 0x1D], 0xBBF350);
    for rva in [0x502A38usize, 0x4FD5FD] { if bytes_at(b() + rva, 6) != old { return Err(format!("site {rva:#x} mismatch")); } }
    let mut n = vec![0xD8, 0x1D]; n.extend(le(super::fixes::pool_f32(r * r)));
    for rva in [0x502A38usize, 0x4FD5FD] { unsafe { write(b() + rva, &n); } }
    Ok(format!("traffic road activation radius 300 -> {r} m"))
}

/// World stream grid: level-1 (60 m cells) keep distance 120 m + load window 300; level-2 (small blocks) keep 25 m.
/// The unload batch in the streaming update has a fixed 61-entry array: keep the steps moderate (clamped).
unsafe fn streaming() -> Result<String, String> {
    let l1 = ini::f32v("World", "StreamingDistance", 120.0);
    let l2 = ini::f32v("World", "StreamingDetailDistance", 25.0);
    if (l1 - 120.0).abs() < 0.5 && (l2 - 25.0).abs() < 0.5 { return Err("vanilla (ini)".into()); }
    if rd_f32(0xBC4460) != 120.0 || rd_f32(0xC4B048) != 300.0 || rd_f32(0xBC4464) != 25.0 { return Err("table mismatch".into()); }
    let l1 = l1.clamp(120.0, 240.0);
    let l2 = l2.clamp(25.0, 100.0);
    unsafe {
        write(b() + 0xBC4460, &l1.to_le_bytes());
        write(b() + 0xC4B048, &(2.0 * l1 + 60.0).max(300.0).to_le_bytes());
        write(b() + 0xBC4464, &l2.to_le_bytes());
    }
    Ok(format!("stream keep: level-1 {l1} m (window {}), level-2 {l2} m (vanilla 120/300/25)", (2.0 * l1 + 60.0).max(300.0)))
}
