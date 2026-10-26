//! Read-only MAIN.EXE update observer for the hash-pinned Nymashock runtime.
//! CPU hook ABI/interrupt handling: tenchu-decomp-glitch-research's
//! tools/tenchu-worker/sim_phase_probe.rs (Mednafen 382ff1b8d293c9a8).
use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

const RAM_BYTES: usize = 0x200000;
const CAPACITY: usize = 32 * 1024 * 1024;
static ACTIVE: AtomicPtr<Probe> = AtomicPtr::new(ptr::null_mut());

struct Probe {
    cpu: *const u8,
    ram: *const u8,
    field: u32,
    cycles: i64,
    stage: u32,
    bytes: Vec<u8>,
    failed: bool,
    error: u64,
    previous: Vec<u8>,
    captured: bool,
    detail: Option<(u32, u32)>,
}

unsafe fn word(base: *const u8, offset: usize) -> u32 {
    unsafe { base.add(offset).cast::<u32>().read_unaligned() }
}

fn interrupted(p: &Probe, pc: u32) -> Option<bool> {
    match unsafe { word(p.cpu, 0x94) } {
        0 => Some(false),
        0x80 => {
            let physical = (pc & 0x1fffffff) as usize;
            if physical > RAM_BYTES - 4 || pc & 3 != 0 {
                return None;
            }
            let cache = 0x194 + 2 * (pc as usize & 0xffc);
            let instruction = unsafe {
                if word(p.cpu, cache) == pc {
                    word(p.cpu, cache + 4)
                } else {
                    word(p.ram, physical)
                }
            };
            Some(instruction >> 26 != 0x12)
        }
        _ => None,
    }
}

extern "C" fn observe(timestamp: i32, pc: u32) {
    if !matches!(
        pc,
        0x8001ada4
            | 0x8004df58
            | 0x800502b4
            | 0x8001a688
            | 0x800181d4
            | 0x800319f8
            | 0x800292a4
            | 0x8003b80c
            | 0x8003be54
            | 0x80039bbc
            | 0x8004a12c
            | 0x8004ba5c
            | 0x8004d350
            | 0x80029368
            | 0x800170c8
            | 0x80017234
            | 0x8001ac88
            | 0x80039bf0
    ) {
        return;
    }
    let address = ACTIVE.load(Ordering::Relaxed);
    if address.is_null() {
        return;
    }
    let p = unsafe { &mut *address };
    if p.failed {
        return;
    }
    if pc == 0x80039bf0
        && !p
            .detail
            .is_some_and(|(start, end)| (start..=end).contains(&p.field))
    {
        return;
    }
    // These executable words distinguish MAIN from MENU/ENDING while their
    // address spaces overlap. The host independently pins the core and disc.
    if unsafe {
        word(p.ram, 0x162a4) != 0x27bdfee8
            || word(p.ram, 0x17030) != 0x27bdffe0
            || word(p.ram, 0x1ada4) != 0x27bdffe8
            || word(p.ram, 0x4df58) != 0x8f8405c8
    } {
        return;
    }
    match interrupted(p, pc) {
        Some(false) => (),
        Some(true) => return,
        None => {
            p.failed = true;
            p.error = (u64::from(pc) << 32) | u64::from(unsafe { word(p.cpu, 0x94) });
            return;
        }
    }
    let stage = unsafe { word(p.ram, 0x97ac4) };
    let initial = pc == 0x8004df58 && stage != p.stage;
    if initial {
        p.stage = stage;
    }
    let snapshot = initial
        || pc == 0x800170c8
        || p.detail
            .is_some_and(|(start, end)| (start..=end).contains(&p.field));
    let size = 56 + if snapshot { RAM_BYTES + 65536 } else { 0 };
    if p.bytes.len() + size > CAPACITY {
        p.failed = true;
        p.error = (u64::from(pc) << 32) | 1;
        return;
    }
    let mut caller = unsafe { word(p.cpu, 4 * 31) };
    if matches!(pc, 0x800170c8 | 0x80017234) {
        let stack = (unsafe { word(p.cpu, 4 * 29) } & 0x1fffffff) as usize;
        if stack < 0x800000 && stack & 3 == 0 && (stack & 0x1fffff) <= RAM_BYTES - 28 {
            caller = unsafe { word(p.ram, (stack & 0x1fffff) + 24) };
        } else {
            p.failed = true;
            p.error = (u64::from(pc) << 32) | stack as u64;
            return;
        }
    }
    let values = unsafe {
        [
            56,
            pc,
            p.field,
            word(p.ram, 0x976b4) & 0xffff,
            0,
            0,
            word(p.ram, 0x976b0),
            stage,
            word(p.ram, 0xbe6d0) & 0xffff,
            word(p.ram, 0xc79e0),
            word(p.ram, 0xc6400),
            caller,
            word(p.cpu, 8),
            word(p.cpu, 16),
        ]
    };
    let start = p.bytes.len();
    for value in values {
        p.bytes.extend_from_slice(&value.to_le_bytes());
    }
    p.bytes[start + 16..start + 24]
        .copy_from_slice(&(p.cycles + i64::from(timestamp)).to_le_bytes());
    if snapshot {
        let ram = unsafe { std::slice::from_raw_parts(p.ram, RAM_BYTES) };
        let count_at = p.bytes.len();
        p.bytes.extend_from_slice(&0u32.to_le_bytes());
        let mut count = 0u32;
        let mut offset = 0;
        while offset < RAM_BYTES {
            if p.captured && ram[offset..offset + 256] == p.previous[offset..offset + 256] {
                offset += 256;
                continue;
            }
            let begin = offset;
            offset += 256;
            while offset < RAM_BYTES
                && (!p.captured || ram[offset..offset + 256] != p.previous[offset..offset + 256])
            {
                offset += 256;
            }
            p.bytes.extend_from_slice(&(begin as u32).to_le_bytes());
            p.bytes
                .extend_from_slice(&((offset - begin) as u32).to_le_bytes());
            p.bytes.extend_from_slice(&ram[begin..offset]);
            p.previous[begin..offset].copy_from_slice(&ram[begin..offset]);
            count += 1;
        }
        p.bytes[count_at..count_at + 4].copy_from_slice(&count.to_le_bytes());
        p.captured = true;
    }
    let size = (p.bytes.len() - start) as u32;
    p.bytes[start..start + 4].copy_from_slice(&size.to_le_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn retail_probe_begin(cpu: *const u8, ram: *const u8) -> *const () {
    if cpu.is_null() || ram.is_null() || !ACTIVE.load(Ordering::SeqCst).is_null() {
        return ptr::null();
    }
    let p = Box::new(Probe {
        cpu,
        ram,
        field: 0,
        cycles: 0,
        stage: u32::MAX,
        bytes: Vec::with_capacity(CAPACITY),
        failed: false,
        error: 0,
        previous: vec![0; RAM_BYTES],
        captured: false,
        detail: std::env::var("TENCHU_RETAIL_PHASE_FIELDS")
            .ok()
            .map(|value| {
                let (start, end) = value
                    .split_once(':')
                    .expect("TENCHU_RETAIL_PHASE_FIELDS needs START:END");
                let (start, end) = (
                    start.parse().expect("Invalid first detail field"),
                    end.parse().expect("Invalid last detail field"),
                );
                assert!(start <= end, "Invalid detail field range");
                (start, end)
            }),
    });
    ACTIVE.store(Box::into_raw(p), Ordering::SeqCst);
    observe as *const ()
}

#[unsafe(no_mangle)]
pub extern "C" fn retail_probe_field(field: u32, cycles: i64) -> i32 {
    let p = ACTIVE.load(Ordering::SeqCst);
    if p.is_null() {
        return -1;
    }
    let p = unsafe { &mut *p };
    if p.failed || field != p.field + 1 || !p.bytes.is_empty() {
        return -1;
    }
    p.field = field;
    p.cycles = cycles;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn retail_probe_size() -> i32 {
    let p = ACTIVE.load(Ordering::SeqCst);
    if p.is_null() {
        return -1;
    }
    let p = unsafe { &*p };
    if p.failed { -1 } else { p.bytes.len() as i32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn retail_probe_error() -> u64 {
    let p = ACTIVE.load(Ordering::SeqCst);
    if p.is_null() {
        u64::MAX
    } else {
        unsafe { (*p).error }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retail_probe_take(output: *mut u8, size: usize) -> i32 {
    let p = ACTIVE.load(Ordering::SeqCst);
    if p.is_null() || output.is_null() {
        return -1;
    }
    let p = unsafe { &mut *p };
    if p.failed || size != p.bytes.len() {
        return -1;
    }
    unsafe {
        ptr::copy_nonoverlapping(p.bytes.as_ptr(), output, size);
    }
    p.bytes.clear();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn retail_probe_end() {
    let p = ACTIVE.swap(ptr::null_mut(), Ordering::SeqCst);
    if !p.is_null() {
        unsafe {
            drop(Box::from_raw(p));
        }
    }
}
