use crate::win::*;
use core::ffi::c_void;

pub struct Section { pub name: [u8; 8], pub start: usize, pub size: usize }

pub fn base() -> usize { unsafe { GetModuleHandleA(core::ptr::null()) as usize } }

pub fn sections() -> Vec<Section> {
    unsafe {
        let b = base();
        let pe = b + *((b + 0x3C) as *const u32) as usize;
        let n = *((pe + 6) as *const u16) as usize;
        let opt = *((pe + 20) as *const u16) as usize;
        let first = pe + 24 + opt;
        (0..n).map(|i| {
            let s = first + i * 40;
            let mut name = [0u8; 8];
            name.copy_from_slice(core::slice::from_raw_parts(s as *const u8, 8));
            Section { name, start: b + *((s + 12) as *const u32) as usize, size: *((s + 8) as *const u32) as usize }
        }).collect()
    }
}

pub fn section(name: &[u8]) -> Option<(usize, usize)> {
    sections().into_iter().find(|s| s.name.starts_with(name)).map(|s| (s.start, s.size))
}

/// Pattern like "D9 05 ?? ?? ?? ?? A2". Returns all matches in [start, start+size).
pub fn find_all(pat: &str, start: usize, size: usize) -> Vec<usize> {
    let p: Vec<Option<u8>> = pat.split_whitespace()
        .map(|t| if t.starts_with('?') { None } else { Some(u8::from_str_radix(t, 16).unwrap()) }).collect();
    let mut out = Vec::new();
    if p.is_empty() || size < p.len() { return out; }
    let hay = unsafe { core::slice::from_raw_parts(start as *const u8, size) };
    let first = p[0];
    'outer: for i in 0..=(size - p.len()) {
        if let Some(f) = first { if hay[i] != f { continue; } }
        for (j, b) in p.iter().enumerate() {
            if let Some(v) = b { if hay[i + j] != *v { continue 'outer; } }
        }
        out.push(start + i);
    }
    out
}

pub fn find_text(pat: &str) -> Vec<usize> {
    match section(b".text") { Some((s, n)) => find_all(pat, s, n), None => Vec::new() }
}

pub unsafe fn write(addr: usize, bytes: &[u8]) {
    unsafe {
        let mut old = 0u32;
        VirtualProtect(addr as *mut c_void, bytes.len(), PAGE_EXECUTE_READWRITE, &mut old);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
        VirtualProtect(addr as *mut c_void, bytes.len(), old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), addr as *const c_void, bytes.len());
    }
}

pub unsafe fn read<T: Copy>(addr: usize) -> T { unsafe { core::ptr::read_unaligned(addr as *const T) } }

pub fn bytes_at(addr: usize, n: usize) -> Vec<u8> { unsafe { core::slice::from_raw_parts(addr as *const u8, n).to_vec() } }

pub fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ") }

/// Permanently allocated executable/data memory for caves and constants.
pub fn alloc(size: usize) -> usize {
    unsafe { VirtualAlloc(core::ptr::null_mut(), size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as usize }
}

/// Write a 5-byte JMP rel32 from `at` to `to`, padding the rest of `len` with NOPs.
pub unsafe fn jmp(at: usize, to: usize, len: usize) {
    let mut v = vec![0xE9u8];
    v.extend_from_slice(&((to as isize - (at as isize + 5)) as i32).to_le_bytes());
    while v.len() < len { v.push(0x90); }
    unsafe { write(at, &v) }
}

/// A bump allocator for small static constants that patched code can reference by absolute address.
pub struct Pool { cur: usize, end: usize }
impl Pool {
    pub fn new(size: usize) -> Self { let a = alloc(size); Pool { cur: a, end: a + size } }
    pub fn put_f32(&mut self, v: f32) -> usize { let a = self.cur; unsafe { *(a as *mut f32) = v; } self.cur += 4; assert!(self.cur <= self.end); a }
    pub fn put_bytes(&mut self, b: &[u8]) -> usize { let a = self.cur; unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), a as *mut u8, b.len()); } self.cur += (b.len() + 15) & !15; assert!(self.cur <= self.end); a }
}
