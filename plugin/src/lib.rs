//! SaboteurFix: dinput8.dll proxy that applies in-memory fixes to The Saboteur at load time.
//! The exe on disk stays untouched. Every fix is located by signature or verified bytes and can be turned off in SaboteurFix.ini.
#![allow(clippy::missing_safety_doc)]
mod win;
mod mem;
mod ini;
mod log;
mod fixes;
mod fixes_extra;
mod window;
mod fixes3;
mod inject;
mod fixes4;
mod fixes5;
mod fixes6;
mod fixes7;
mod fixes8;

use core::ffi::c_void;
use win::*;

static mut REAL_DI8_CREATE: *mut c_void = core::ptr::null_mut();

unsafe fn load_real_dinput8() {
    unsafe {
        let mut buf = [0u8; 300];
        let n = GetSystemDirectoryA(buf.as_mut_ptr(), 260) as usize;
        let tail = b"\\dinput8.dll\0";
        buf[n..n + tail.len()].copy_from_slice(tail);
        let h = LoadLibraryA(buf.as_ptr());
        if !h.is_null() {
            REAL_DI8_CREATE = GetProcAddress(h, b"DirectInput8Create\0".as_ptr());
        }
    }
}

type Di8CreateFn = unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void, *mut c_void) -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "system" fn proxy_DirectInput8Create(
    hinst: *mut c_void, ver: u32, riid: *const c_void, out: *mut *mut c_void, outer: *mut c_void,
) -> i32 {
    unsafe {
        if REAL_DI8_CREATE.is_null() { load_real_dinput8(); }
        if REAL_DI8_CREATE.is_null() { return 0x8007_007Eu32 as i32; } // ERROR_MOD_NOT_FOUND
        let f: Di8CreateFn = core::mem::transmute(REAL_DI8_CREATE);
        let r = f(hinst, ver, riid, out, outer);
        if r >= 0 && !out.is_null() { inject::on_di8(*out); }
        r
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(hinst: HMODULE, reason: u32, _r: *mut c_void) -> BOOL {
    if reason == 1 {
        unsafe {
            log::init(hinst);
            ini::init(hinst);
            fixes::apply_all();
            window::start();
            inject::init(hinst);
        }
    }
    1
}
