#![no_std]
#![no_main]


mod calibration;
mod init;


use core::panic::PanicInfo;

use atsaml21g18b::Peripherals;
use cortex_m_rt::entry;


#[panic_handler]
fn panicked(_reason: &PanicInfo) -> ! {
    loop {
    }
}


#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take()
        .expect("peripherals already taken?!");

    crate::init::initialize_microcontroller(&mut peripherals);

    loop {
    }
}
