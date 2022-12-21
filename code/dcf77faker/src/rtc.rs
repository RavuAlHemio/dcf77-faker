//! Code to control a real-time counter.


use atsaml21g18b::{Interrupt, Peripherals};
use cortex_m::peripheral::NVIC;


/// Enables the clocks for RTC.
pub(crate) fn enable_clock(peripherals: &mut Peripherals) {
    // enable CLK_RTC_APB
    peripherals.MCLK.apbamask.modify(|_, w| w
        .rtc_().set_bit()
    );

    // CLK_RTC_OSC always enabled (source configured directly in OSC32KCTRL)
}


/// Sets up RTC.
pub(crate) fn setup_rtc(peripherals: &mut Peripherals) {
    enable_clock(peripherals);

    // raw frequency: 32_768 Hz
    // prescaler: 1/1024
    // final frequency: 32 Hz
    // we need to act every second => a 16-bit counter is enough
    // => use RTC mode 1
    let register_block = peripherals.RTC.mode1();

    // reset RTC
    register_block.ctrla.modify(|_, w| w
        .swrst().set_bit()
    );
    while register_block.syncbusy.read().swrst().bit_is_set() {
    }

    // basic configuration
    register_block.ctrla.modify(|_, w| w
        .mode().count16() // mode 1 (16-bit counter)
        .prescaler().div1024() // prescaler to 1/1024
        .enable().clear_bit() // don't start yet
    );

    // set period to 32
    register_block.per.modify(|_, w| w
        .per().variant(32)
    );
    while register_block.syncbusy.read().per().bit_is_set() {
    }

    // interrupt on overflow
    register_block.intenset.modify(|_, w| w
        .ovf().set_bit()
    );

    // start
    register_block.ctrla.modify(|_, w| w
        .enable().set_bit() // start
    );
    while register_block.syncbusy.read().enable().bit_is_set() {
    }
}


/// Enable the RTC interrupt.
pub(crate) fn enable_interrupt() {
    unsafe {
        NVIC::unmask(Interrupt::RTC)
    }
}
