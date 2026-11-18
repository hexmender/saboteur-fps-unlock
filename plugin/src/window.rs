//! Borderless fullscreen: when the game runs with Windowed 1, turn its window into a borderless popup covering the monitor.
use crate::win::*;
use crate::{ini, logf};
use core::ffi::c_void;

static mut FOUND: *mut c_void = core::ptr::null_mut();

unsafe extern "system" fn enum_cb(h: *mut c_void, _l: isize) -> BOOL {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, &mut pid);
        if pid == GetCurrentProcessId() && IsWindowVisible(h) != 0 && GetWindow(h, 4 /*GW_OWNER*/).is_null() {
            let mut r = RECT { l: 0, t: 0, r: 0, b: 0 };
            GetWindowRect(h, &mut r);
            if r.r - r.l > 640 { FOUND = h; return 0; }
        }
        1
    }
}

#[link(name = "winmm")]
unsafe extern "system" { fn timeBeginPeriod(p: u32) -> u32; }

unsafe extern "system" fn worker(_: *mut c_void) -> u32 {
    unsafe {
        if ini::boolv("Main", "HighResTimer", true) {
            timeBeginPeriod(1);
            logf!("[ok]   HighResTimer: timeBeginPeriod(1) (faster Sleep-based loading loops)");
        }
        if !ini::boolv("Main", "BorderlessFullscreen", true) { return 0; }
        // The game creates/resizes its window during boot: enforce the popup only during boot, only while the
        // window is foreground and not minimized, and stop once it has stayed correct for ~5 s.
        // (Earlier versions kept enforcing every 3 s for ~100 min and re-showed the window while it was
        // minimized by alt-tab -> swapchain recreation fights -> VK_ERROR_DEVICE_LOST / system hang on AMD.)
        let mut done = 0u32;
        let mut stable = 0u32;
        for _ in 0..240u32 {
            Sleep(500);
            FOUND = core::ptr::null_mut();
            EnumWindows(enum_cb, 0);
            let h = FOUND;
            if h.is_null() { continue; }
            if IsIconic(h) != 0 || GetForegroundWindow() != h { stable = 0; continue; }
            let style = GetWindowLongA(h, -16) as u32;
            let mon = MonitorFromWindow(h, 2);
            let mut mi = MONITORINFO { size: core::mem::size_of::<MONITORINFO>() as u32, mon: RECT { l: 0, t: 0, r: 0, b: 0 }, work: RECT { l: 0, t: 0, r: 0, b: 0 }, flags: 0 };
            GetMonitorInfoA(mon, &mut mi);
            let mut r = RECT { l: 0, t: 0, r: 0, b: 0 };
            GetWindowRect(h, &mut r);
            let want = 0x8000_0000u32 | 0x1000_0000; // WS_POPUP | WS_VISIBLE
            let ok_style = style & 0x00C4_0000 == 0; // no caption / thickframe
            let ok_rect = r.l == mi.mon.l && r.t == mi.mon.t && r.r == mi.mon.r && r.b == mi.mon.b;
            if !(ok_style && ok_rect) {
                SetWindowLongA(h, -16, (want | (style & 0x0400_0000 /*WS_CLIPSIBLINGS*/)) as i32);
                SetWindowLongA(h, -20, (GetWindowLongA(h, -20) as u32 & !(0x0000_0200 | 0x0000_0001 | 0x0002_0000)) as i32); // drop client/dlg/static edge
                SetWindowPos(h, core::ptr::null_mut(), mi.mon.l, mi.mon.t, mi.mon.r - mi.mon.l, mi.mon.b - mi.mon.t, 0x0020 | 0x0040 | 0x0004); // FRAMECHANGED|SHOWWINDOW|NOZORDER
                done += 1;
                if done <= 3 { logf!("[ok]   Borderless: window set to {}x{} popup", mi.mon.r - mi.mon.l, mi.mon.b - mi.mon.t); }
                stable = 0;
            } else {
                stable += 1;
                if stable >= 10 && done > 0 { logf!("[ok]   Borderless: stable, watcher stopped"); return 0; }
            }
        }
        0
    }
}

pub fn start() {
    unsafe { CreateThread(core::ptr::null_mut(), 0, worker, core::ptr::null_mut(), 0, core::ptr::null_mut()); }
}
