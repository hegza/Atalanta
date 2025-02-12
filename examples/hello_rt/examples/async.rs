//! Set up 4 timers to trigger one after each other. Assert that all interrupts
//! were fired.
#![no_main]
#![no_std]
#![allow(static_mut_refs)]
#![allow(non_snake_case)]

use bsp::{
    clic::Clic,
    mtimer::{Duration, MTimer},
    nested_interrupt,
    riscv::{self, asm::wfi},
    rt::entry,
    sprint, sprintln,
    tb::signal_pass,
    uart::*,
    Interrupt, CPU_FREQ, NOPS_PER_SEC,
};
use hello_rt::{function, print_example_name, setup_irq, tear_irq, UART_BAUD};

const INTERVAL: u64 = if cfg!(feature = "rtl-tb") {
    0x100
} else {
    // Just enough to be able to tell the timer's apart
    NOPS_PER_SEC as u64 / 2
};

static mut IRQ_COUNT: u64 = 0;

#[entry]
fn main() -> ! {
    let mut serial = ApbUart::init(CPU_FREQ, UART_BAUD);
    print_example_name!();

    // Set level bits to 8
    Clic::smclicconfig().set_mnlbits(8);

    // Setup timers
    setup_irq(Interrupt::MachineTimer);

    // Use mtimer for timeout
    let mut mtimer = MTimer::instance().into_oneshot();

    sprintln!("dispatching timer...");

    // Enable interrupts globally
    unsafe { riscv::interrupt::enable() };

    // Wait for timeout from timer
    while unsafe { IRQ_COUNT < 3 } {
        mtimer.start(Duration::from_ticks(INTERVAL));
        wfi();
    }

    riscv::interrupt::disable();

    // Tear down after timeout
    tear_irq(Interrupt::MachineTimer);

    signal_pass(Some(&mut serial));
    loop {
        // Wait for interrupt
        wfi();
    }
}

#[nested_interrupt]
fn MachineTimer() {
    sprint!("enter {}", function!());
    let irq_code = (riscv::register::mcause::read().bits() & 0xfff) as u16;
    sprintln!(" code: {}", irq_code);

    unsafe { IRQ_COUNT += 1 };
    MTimer::instance().reset();
}
