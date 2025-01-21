//! Blink the led at an arbitrary frequency
#![no_main]
#![no_std]

use panic_halt as _;
use riscv_rt::entry;
use rt_minimal::{asm_delay, read_u8, write_u32, write_u8, NOPS_PER_SEC};

const LED_ADDR: usize = 0x0003_0008;

#[inline(never)]
fn blinky() {
    loop {
        write_u32(LED_ADDR, 1);
        asm_delay(NOPS_PER_SEC / 2);
        write_u32(LED_ADDR, 0);
        asm_delay(NOPS_PER_SEC / 2);
    }
}

#[entry]
fn main() -> ! {
    blinky();
    loop {}
}
