//! Debug-only input injection for automated testing ([Debug] InputInjection=1).
//! Hooks IDirectInput8::CreateDevice -> IDirectInputDevice8::GetDeviceState and merges synthetic input
//! read from "SaboteurFix_input.txt" next to the dll:  lines "K <scan hex>" = key held, "M <dx> <dy>" = mouse delta (consumed once),
//! "B <n>" = mouse button n held. The file is re-read every 30 ms.
use crate::win::*;
use crate::{ini, logf};
use core::ffi::c_void;

type GetDeviceStateFn = unsafe extern "system" fn(*mut c_void, u32, *mut u8) -> i32;
type CreateDeviceFn = unsafe extern "system" fn(*mut c_void, *const c_void, *mut *mut c_void, *mut c_void) -> i32;

static mut ORIG_GDS: Option<GetDeviceStateFn> = None;
static mut ORIG_CD: Option<CreateDeviceFn> = None;
static mut KEYS: [u8; 256] = [0; 256];
static mut BTN: [u8; 8] = [0; 8];
static mut MDX: i32 = 0;
static mut MDY: i32 = 0;
static mut PATH: Vec<u8> = Vec::new();

unsafe fn patch_vtbl(obj: *mut c_void, idx: usize, f: usize) -> usize {
    unsafe {
        let vt = *(obj as *const *mut usize);
        let slot = vt.add(idx);
        let old = *slot;
        if old == f { return old; }
        let mut o = 0u32;
        VirtualProtect(slot as *mut c_void, 4, PAGE_EXECUTE_READWRITE, &mut o);
        *slot = f;
        VirtualProtect(slot as *mut c_void, 4, o, &mut o);
        old
    }
}

unsafe extern "system" fn hk_gds(this: *mut c_void, cb: u32, data: *mut u8) -> i32 {
    unsafe {
        let r = (ORIG_GDS.unwrap())(this, cb, data);
        if r < 0 || data.is_null() { return r; }
        if cb == 256 {
            for i in 0..256 { if KEYS[i] != 0 { *data.add(i) = 0x80; } }
        } else if cb == 0x14 || cb == 0x10 {
            let p = data as *mut i32;
            *p += MDX; *p.add(1) += MDY; MDX = 0; MDY = 0;
            for i in 0..4 { if BTN[i] != 0 { *data.add(12 + i) = 0x80; } }
        }
        r
    }
}

unsafe extern "system" fn hk_cd(this: *mut c_void, guid: *const c_void, out: *mut *mut c_void, outer: *mut c_void) -> i32 {
    unsafe {
        let r = (ORIG_CD.unwrap())(this, guid, out, outer);
        if r >= 0 && !out.is_null() && !(*out).is_null() {
            let old = patch_vtbl(*out, 9, hk_gds as usize);
            if old != hk_gds as usize { ORIG_GDS = Some(core::mem::transmute(old)); }
        }
        r
    }
}

/// Called with the freshly created IDirectInput8 object.
pub unsafe fn on_di8(obj: *mut c_void) {
    unsafe {
        if (*core::ptr::addr_of!(PATH)).is_empty() || obj.is_null() { return; }
        let old = patch_vtbl(obj, 3, hk_cd as usize);
        if old != hk_cd as usize { ORIG_CD = Some(core::mem::transmute(old)); }
        logf!("[dbg]  input injection hooked");
    }
}

unsafe extern "system" fn reader(_: *mut c_void) -> u32 {
    unsafe {
        let mut last_m = String::new();
        loop {
            Sleep(30);
            let path = &*core::ptr::addr_of!(PATH);
            let s = std::fs::read_to_string(std::str::from_utf8(&path[..path.len() - 1]).unwrap_or("")).unwrap_or_default();
            let mut k = [0u8; 256]; let mut b = [0u8; 8];
            for line in s.lines() {
                let t: Vec<&str> = line.split_whitespace().collect();
                match t.as_slice() {
                    ["K", sc] => if let Ok(v) = usize::from_str_radix(sc, 16) { if v < 256 { k[v] = 1; } },
                    ["B", n] => if let Ok(v) = n.parse::<usize>() { if v < 8 { b[v] = 1; } },
                    ["M", id, dx, dy] => if *id != last_m.as_str() {
                        last_m = id.to_string();
                        MDX += dx.parse::<i32>().unwrap_or(0); MDY += dy.parse::<i32>().unwrap_or(0);
                    },
                    _ => {}
                }
            }
            KEYS = k; BTN = b;
        }
    }
}

pub fn init(hinst: HMODULE) {
    if !ini::boolv("Debug", "InputInjection", false) { return; }
    unsafe {
        let mut p = crate::ini::dir_of(hinst);
        p.extend_from_slice(b"SaboteurFix_input.txt\0");
        PATH = p;
        CreateThread(core::ptr::null_mut(), 0, reader, core::ptr::null_mut(), 0, core::ptr::null_mut());
    }
}
