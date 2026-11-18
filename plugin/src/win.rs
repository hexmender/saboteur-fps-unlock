//! Minimal Win32 FFI (no external crates).
#![allow(non_snake_case, clippy::upper_case_acronyms)]
use core::ffi::c_void;

pub type HMODULE = *mut c_void;
pub type BOOL = i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleA(name: *const u8) -> HMODULE;
    pub fn GetModuleFileNameA(m: HMODULE, buf: *mut u8, size: u32) -> u32;
    pub fn LoadLibraryA(name: *const u8) -> HMODULE;
    pub fn GetProcAddress(m: HMODULE, name: *const u8) -> *mut c_void;
    pub fn GetSystemDirectoryA(buf: *mut u8, size: u32) -> u32;
    pub fn VirtualProtect(addr: *mut c_void, size: usize, new: u32, old: *mut u32) -> BOOL;
    pub fn VirtualAlloc(addr: *mut c_void, size: usize, ty: u32, prot: u32) -> *mut c_void;
    pub fn FlushInstructionCache(proc_: *mut c_void, addr: *const c_void, size: usize) -> BOOL;
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn GetPrivateProfileStringA(app: *const u8, key: *const u8, def: *const u8, out: *mut u8, size: u32, file: *const u8) -> u32;
    pub fn CreateFileA(name: *const u8, access: u32, share: u32, sa: *mut c_void, disp: u32, flags: u32, tmpl: *mut c_void) -> *mut c_void;
    pub fn WriteFile(h: *mut c_void, buf: *const u8, n: u32, written: *mut u32, ov: *mut c_void) -> BOOL;
    pub fn CloseHandle(h: *mut c_void) -> BOOL;
    pub fn SetFilePointer(h: *mut c_void, dist: i32, high: *mut i32, method: u32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn CreateThread(sa: *mut c_void, stack: usize, start: unsafe extern "system" fn(*mut c_void) -> u32, param: *mut c_void, flags: u32, id: *mut u32) -> *mut c_void;
    pub fn Sleep(ms: u32);
    pub fn GetCurrentProcessId() -> u32;
}

#[repr(C)] pub struct RECT { pub l: i32, pub t: i32, pub r: i32, pub b: i32 }
#[repr(C)] pub struct MONITORINFO { pub size: u32, pub mon: RECT, pub work: RECT, pub flags: u32 }

#[link(name = "user32")]
unsafe extern "system" {
    pub fn EnumWindows(cb: unsafe extern "system" fn(*mut c_void, isize) -> BOOL, l: isize) -> BOOL;
    pub fn GetWindowThreadProcessId(h: *mut c_void, pid: *mut u32) -> u32;
    pub fn IsWindowVisible(h: *mut c_void) -> BOOL;
    pub fn GetWindow(h: *mut c_void, cmd: u32) -> *mut c_void;
    pub fn GetWindowLongA(h: *mut c_void, idx: i32) -> i32;
    pub fn SetWindowLongA(h: *mut c_void, idx: i32, v: i32) -> i32;
    pub fn SetWindowPos(h: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> BOOL;
    pub fn MonitorFromWindow(h: *mut c_void, flags: u32) -> *mut c_void;
    pub fn GetMonitorInfoA(m: *mut c_void, mi: *mut MONITORINFO) -> BOOL;
    pub fn GetWindowRect(h: *mut c_void, r: *mut RECT) -> BOOL;
    pub fn IsIconic(h: *mut c_void) -> BOOL;
    pub fn GetForegroundWindow() -> *mut c_void;
}

pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;
pub const MEM_COMMIT: u32 = 0x1000;
pub const MEM_RESERVE: u32 = 0x2000;
