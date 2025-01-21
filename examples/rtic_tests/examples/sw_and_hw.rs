#![no_main]
#![no_std]

use bsp::{rt::pre_init, uart::init_uart};

#[pre_init]
unsafe fn pre_init() {
    init_uart(bsp::CPU_FREQ, 9600);
}

static mut LO_DONE: usize = 0;
static mut HI_DONE: usize = 0;

#[rtic::app(device = bsp, dispatchers = [])]
mod app {
    use bsp::{interrupt::Interrupt, sprintln};
    use core::arch::asm;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    // do nothing in init
    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        sprintln!("init");
        unsafe {
            crate::HI_DONE = 0;
            crate::LO_DONE = 0;
        }

        //high_task::spawn().ok().unwrap();
        unsafe { bsp::clic::CLIC::ip(Interrupt::Dma0).pend() };
        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        let mut try_count = 0;
        sprintln!("idle");
        loop {
            if unsafe { crate::HI_DONE != 0 } && unsafe { crate::LO_DONE != 0 } {
                bsp::tb::signal_pass(true);
            }
            for _ in 0..5000000 {
                // burn cycles
                unsafe { asm!("nop") };
            }
            try_count += 1;
            if try_count >= 3 {
                sprintln!(
                    "lo done: {}, hi done: {}",
                    unsafe { crate::LO_DONE },
                    unsafe { crate::HI_DONE }
                );
                bsp::tb::signal_fail(true);
            }
        }
    }

    #[task(binds = Dma0, priority = 2)]
    fn high_task(_: high_task::Context) {
        sprintln!("high_task enter");

        // Enqueue low prio task
        //low_task::spawn().unwrap();
        unsafe { bsp::clic::CLIC::ip(Interrupt::Dma1).pend() };
        /*unsafe { bsp::riscv::interrupt::enable() };
        unsafe { core::arch::asm!("csrwi 0x347, 1") };*/

        for _ in 0..5000000 {
            // burn cycles
            unsafe { asm!("nop") };
        }

        unsafe { crate::HI_DONE += 1 };
        sprintln!("high_task leave");
    }

    #[task(binds = Dma1, priority = 5)]
    fn low_task(_: low_task::Context) {
        sprintln!("low_task enter");

        for _ in 0..5000000 {
            // burn cycles
            unsafe { asm!("nop") };
        }

        unsafe { crate::LO_DONE += 1 };
        sprintln!("low_task leave");
    }
}
