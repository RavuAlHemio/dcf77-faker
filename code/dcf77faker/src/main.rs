#![no_std]
#![no_main]


mod calibration;
mod dcf77;
mod i2c_controller;
mod i2c_display;
mod init;
mod pin;
mod pwm;
mod rtc;
mod sync_vcell;
mod tick;


use core::panic::PanicInfo;

use atsaml21g18b::{interrupt, Peripherals};
use cortex_m_rt::entry;

use crate::pin::PeripheralIndex;


#[inline]
fn noppage() {
    for _ in 0..65536 {
        cortex_m::asm::nop();
    }
}


#[panic_handler]
fn panicked(_reason: &PanicInfo) -> ! {
    let peripherals = unsafe {
        // ain't no rest for the wicked
        Peripherals::steal()
    };

    // set up for blinky LED
    board_pin!(set_io, peripherals, PA, 27);
    board_pin!(make_output, peripherals, PA, 27);

    loop {
        board_pin!(set_high, peripherals, PA, 27);
        noppage();
        board_pin!(set_low, peripherals, PA, 27);
        noppage();
    }
}


#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take()
        .expect("peripherals already taken?!");

    crate::init::initialize_microcontroller(&mut peripherals);

    // set pins as I/O:
    // PA16 = input with pull-up (reset-seconds button)
    // PA17 = input with pull-up (increment-minute button)
    // PA18 = input with pull-up (increment-hour button)
    // PA27 = output (LED)
    board_pin!(set_io, peripherals, PA, 16, 17, 18, 27);
    board_pin!(make_input, peripherals, PA, 16, 17, 18);
    board_pin!(enable_pull, peripherals, PA, 16, 17, 18);
    board_pin!(set_high, peripherals, PA, 16, 17, 18);
    board_pin!(make_output, peripherals, PA, 27);

    // hand over pins to peripherals:
    // PA04 = TCC0/WO[0] (E)
    // PA08 = SERCOM0/PAD[0] (C)
    // PA09 = SERCOM0/PAD[1] (C)
    board_pin!(set_peripheral, peripherals, PA, 4, 8, 9);
    board_pin!(select_peripheral, peripherals, PeripheralIndex::E, PA, 4);
    board_pin!(select_peripheral, peripherals, PeripheralIndex::C, PA, 8, 9);

    loop {
    }
}


#[interrupt]
fn RTC() {
    // TODO
}
