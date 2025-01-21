#![no_std]

use core::arch::{asm, global_asm};
use core::ptr;

pub const CPU_FREQ: u32 = match () {
    #[cfg(feature = "rtl-tb")]
    () => 100_000_000,
    #[cfg(not(feature = "rtl-tb"))]
    () => 30_000_000,
};
// Experimentally found value for how to adjust for real-time
const fn nop_mult() -> u32 {
    match () {
        #[cfg(debug_assertions)]
        () => 60 / 1,
        #[cfg(not(debug_assertions))]
        () => 60 / 13,
    }
}
pub const NOPS_PER_SEC: u32 = CPU_FREQ / nop_mult();

pub const UART_BAUD: u32 = if cfg!(feature = "rtl-tb") {
    3_000_000
} else {
    9600
};

pub fn asm_delay(t: u32) {
    for _ in 0..t {
        unsafe { asm!("nop") }
    }
}

#[inline(always)]
pub fn read_u8(addr: usize) -> u8 {
    unsafe { ptr::read_volatile(addr as *const _) }
}

#[inline(always)]
pub fn read_u32(addr: usize) -> u32 {
    unsafe { ptr::read_volatile(addr as *const _) }
}

#[inline(always)]
pub fn write_u8(addr: usize, val: u8) {
    unsafe { ptr::write_volatile(addr as *mut _, val) };
}

#[inline(always)]
pub fn write_u32(addr: usize, val: u32) {
    unsafe {
        ptr::write_volatile(addr as *mut _, val);
    }
}
