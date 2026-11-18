use crate::win::*;
use core::ffi::c_void;

static mut H: *mut c_void = core::ptr::null_mut();

pub unsafe fn init(hinst: HMODULE) {
    unsafe {
        let mut p = crate::ini::dir_of(hinst);
        p.extend_from_slice(b"SaboteurFix.log\0");
        // CREATE_ALWAYS = 2, GENERIC_WRITE = 0x40000000, FILE_SHARE_READ = 1
        H = CreateFileA(p.as_ptr(), 0x4000_0000, 1, core::ptr::null_mut(), 2, 0x80, core::ptr::null_mut());
        line("SaboteurFix 2.0.0 loaded");
    }
}

pub fn line(s: &str) {
    unsafe {
        if H.is_null() || H as isize == -1 { return; }
        let mut w = 0u32;
        let line = format!("{s}\r\n");
        WriteFile(H, line.as_ptr(), line.len() as u32, &mut w, core::ptr::null_mut());
    }
}

#[macro_export]
macro_rules! logf {
    ($($t:tt)*) => { $crate::log::line(&format!($($t)*)) };
}
