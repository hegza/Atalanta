//! Tests that nested interrupts work as expected
//!
//! N.b. this test is unsound as is currently relies on the UB of CLIC that
//! interrupts get dispatched in the order of arrival instead of being
//! arbitrated with priority.
#![no_main]
#![no_std]

use core::{arch::asm, ptr};

use bsp::{
    asm_delay,
    clic::{
        intattr::{Polarity, Trig},
        Clic, InterruptNumber,
    },
    interrupt::nested,
    print_example_name, riscv,
    rt::entry,
    sprint, sprintln,
    tb::{signal_fail, signal_pass},
    uart::init_uart,
    Interrupt,
};

static mut LOCK: u8 = 0;

#[entry]
fn main() -> ! {
    init_uart(bsp::CPU_FREQ, 9600);
    print_example_name!();

    // Set level bits to 8
    Clic::smclicconfig().set_mnlbits(8);

    setup_irq(Interrupt::Sixteen, 0x1);
    setup_irq(Interrupt::Seventeen, 0x2);

    unsafe {
        // Raise interrupt threshold in RT-Ibex before enabling interrupts
        // mintthresh = 0x347
        sprintln!("mintthresh <- 0xff");
        asm!("csrw 0x347, {0}", in(reg) 0xff);

        // Enable global interrupts
        riscv::interrupt::enable();

        // No interrupt will fire yet
        sprintln!("lo::pend");
        Clic::ip(Interrupt::Sixteen).pend();
        sprintln!("hi::pend");
        Clic::ip(Interrupt::Seventeen).pend();

        // Lower interrupt threshold in RT-Ibex. Interrupts should fire.
        sprintln!("mintthresh <- 0x0");
        asm!("csrw 0x347, {0}", in(reg) 0x0);
    }

    asm_delay(1000);

    if unsafe { ptr::read_volatile(ptr::addr_of_mut!(LOCK)) } == 2u8 {
        riscv::interrupt::disable();
        tear_irq(Interrupt::Sixteen);
        tear_irq(Interrupt::Seventeen);
        signal_pass(true)
    } else {
        riscv::interrupt::disable();
        tear_irq(Interrupt::Sixteen);
        tear_irq(Interrupt::Seventeen);
        signal_fail(true)
    }
}

#[export_name = "DefaultHandler"]
fn interrupt_handler() {
    let irq_n = (riscv::register::mcause::read().bits() & 0xfff) as u16;
    let irq = unsafe { Interrupt::from_number(irq_n).unwrap_unchecked() };

    unsafe {
        sprintln!("irq {}: enter", irq_n);
        sprintln!(
            "is_pending lo:{}, hi:{}",
            Clic::ip(Interrupt::Sixteen).is_pending(),
            Clic::ip(Interrupt::Seventeen).is_pending()
        );
    }

    match irq {
        Interrupt::Sixteen => {
            sprintln!("lo: enter");

            sprintln!("lo: waiting for hi to set lock");
            unsafe { nested(|| while ptr::read_volatile(ptr::addr_of_mut!(LOCK)) == 0u8 {}) }

            sprintln!("lo: lock has updated, continuing");

            unsafe { ptr::write_volatile(ptr::addr_of_mut!(LOCK), 2u8) };

            sprintln!("lo: leave");
        }
        Interrupt::Seventeen => {
            sprintln!("hi: enter");
            unsafe { ptr::write_volatile(ptr::addr_of_mut!(LOCK), 1u8) }
            sprintln!("hi: lock value set");
            sprintln!("hi: leave");
        }
        _ => unreachable!(),
    }
}

#[inline]
fn setup_irq(irq: Interrupt, level: u8) {
    sprintln!("set up IRQ");
    Clic::attr(irq).set_trig(Trig::Edge);
    Clic::attr(irq).set_polarity(Polarity::Pos);
    Clic::attr(irq).set_shv(true);
    Clic::ctl(irq).set_level(level);
    unsafe { Clic::ie(irq).enable() };
}

/// Tear down the IRQ configuration to avoid side-effects for further testing
#[inline]
fn tear_irq(irq: Interrupt) {
    Clic::ie(irq).disable();
    Clic::ctl(irq).set_level(0x0);
    Clic::attr(irq).set_shv(false);
    Clic::attr(irq).set_trig(Trig::Level);
    Clic::attr(irq).set_polarity(Polarity::Pos);
}