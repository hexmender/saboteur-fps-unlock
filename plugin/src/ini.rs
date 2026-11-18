use crate::win::*;

static mut PATH: Vec<u8> = Vec::new();

pub unsafe fn dir_of(hinst: HMODULE) -> Vec<u8> {
    unsafe {
        let mut buf = [0u8; 520];
        let n = GetModuleFileNameA(hinst, buf.as_mut_ptr(), 512) as usize;
        let mut v = buf[..n].to_vec();
        while let Some(&c) = v.last() { if c == b'\\' || c == b'/' { break; } v.pop(); }
        v
    }
}

pub unsafe fn init(hinst: HMODULE) {
    unsafe {
        let mut p = dir_of(hinst);
        p.extend_from_slice(b"SaboteurFix.ini\0");
        PATH = p;
    }
}

pub fn get(section: &str, key: &str, default: &str) -> String {
    unsafe {
        let s = format!("{section}\0");
        let k = format!("{key}\0");
        let d = format!("{default}\0");
        let mut out = [0u8; 256];
        let path = &*core::ptr::addr_of!(PATH);
        let n = GetPrivateProfileStringA(s.as_ptr(), k.as_ptr(), d.as_ptr(), out.as_mut_ptr(), 255, path.as_ptr());
        let v = String::from_utf8_lossy(&out[..n as usize]).to_string();
        let v = v.split(';').next().unwrap_or("").trim().to_string();
        if v.is_empty() { default.to_string() } else { v }
    }
}

pub fn f32v(section: &str, key: &str, default: f32) -> f32 {
    get(section, key, &default.to_string()).parse().unwrap_or(default)
}

pub fn boolv(section: &str, key: &str, default: bool) -> bool {
    let v = get(section, key, if default { "1" } else { "0" });
    matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

pub fn i32v(section: &str, key: &str, default: i32) -> i32 {
    let v = get(section, key, &default.to_string());
    v.parse::<i32>().or_else(|_| v.parse::<f32>().map(|f| f.round() as i32)).unwrap_or(default)
}
