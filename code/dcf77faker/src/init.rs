//! Initialization code.


use atsaml21g18b::Peripherals;


/// The speed of the core clock, timed by XOSC.
pub const CORE_CLOCK_SPEED_HZ: u32 = 31_000_000;


/// The speed of the slow clock, timed by XOSC32K.
pub const SLOW_CLOCK_SPEED_HZ: u32 = 32_768;


/// Sets up the microcontroller's clocks that will be used.
///
/// The following clock setup is used by `dcf77faker`:
///
/// ```
/// ┌────────┐            ┌────────┐            ┌────────┐
/// │ XOSC   │            │ GCG0   │            │ CPU    │
/// │ 31 MHz ├────────────┤ 31 MHz ├─────────┬──┤ 31 MHz │
/// └────────┘            └────────┘         │  └────────┘
///                                          │
///                                          │  ┌──────────────────┐
/// ┌────────────┐        ┌────────────┐     │  │ SERCOM0 (I2C)    │
/// │ XOSC32K    │        │ GCG3       │     ├──┤ core: 31 MHz     │
/// │ 32.768 kHz ├─────┬──┤ 32.768 kHz ├────────┤ slow: 32.768 kHz │
/// └────────────┘     │  └────────────┘     │  └──────────────────┘
///                    │                     │
///                    │                     │  ┌────────────┐
///                    │                     │  │ TCC0 (PWM) │
///                    │                     └──┤ 31 MHz     │
///                    │                        └────────────┘
///                    │
///                    │                        ┌─────────────────────┐
///                    │                        │ RTC (state updates) │
///                    └────────────────────────┤ raw: 32.768 kHz     │
///                                             │ prescaler: 1024     │
///                                             │ scaled: 32 Hz       │
///                                             └─────────────────────┘
/// ```
///
/// 31 MHz has been chosen as the frequency for `XOSC` because it is readily divisible by 77.5 kHz,
/// the modulation frequency of DCF77.
pub(crate) fn setup_clocks(peripherals: &mut Peripherals) {
    // initialize XOSC
    peripherals.OSCCTRL.xoscctrl.modify(|_, w| w
        .ondemand().clear_bit() // run even if not explicitly requested
        .runstdby().set_bit() // run in standby mode too
        .xtalen().clear_bit() // it's a fully-fledged oscillator, not a crystal
    );

    // start XOSC
    peripherals.OSCCTRL.xoscctrl.modify(|_, w| w
        .enable().set_bit()
    );
    while peripherals.OSCCTRL.status.read().xoscrdy().bit_is_clear() {
    }

    // changes to GCLK registers must be synchronized
    // (they are governed by a different clock than the CPU core)
    // => always wait for the corresponding SYNCBUSY register bit to clear

    // plug XOSC into GCG0
    peripherals.GCLK.genctrl[0].modify(|_, w| w
        .divsel().clear_bit() // interpret divisor as DIV, not 2**(DIV+1)
        .div().variant(1) // divide by 1 (= no division)
        .runstdby().set_bit() // run even in standby
        .idc().clear_bit() // no need to improve duty cycle; we are not dividing
        .oe().clear_bit() // no explicit I/O output
        .src().xosc() // take time from XOSC
    );
    while peripherals.GCLK.syncbusy.read().genctrl0().bit_is_set() {
    }

    // turn on GCG0
    peripherals.GCLK.genctrl[0].modify(|_, w| w
        .genen().set_bit()
    );
    while peripherals.GCLK.syncbusy.read().genctrl0().bit_is_set() {
    }

    // GCG0 is always connected to the CPU core (SAM L21 datasheet § 17.1, Note)

    // connect GCG0 as core clock to SERCOM0
    const GCLK_SERCOM0_CORE: usize = 18;
    peripherals.GCLK.pchctrl[GCLK_SERCOM0_CORE].modify(|_, w| w
        .gen().gclk0() // take from GCG0
        .chen().set_bit() // enable
    );

    // connect GCG0 to TCC0
    const GCLK_TCC0: usize = 25;
    peripherals.GCLK.pchctrl[GCLK_TCC0].modify(|_, w| w
        .gen().gclk0() // take from GCG0
        .chen().set_bit() // enable
    );

    // initialize XOSC32K
    peripherals.OSC32KCTRL.xosc32k.modify(|_, w| w
        .ondemand().clear_bit() // run even if not explicitly requested
        .runstdby().set_bit() // run in standby mode too
        .xtalen().clear_bit() // it's a fully-fledged oscillator, not a crystal
        .en32k().set_bit() // enable 32kHz output
    );

    // start XOSC32K
    peripherals.OSC32KCTRL.xosc32k.modify(|_, w| w
        .enable().set_bit()
    );

    // plug XOSC32K into GCG3
    peripherals.GCLK.genctrl[3].modify(|_, w| w
        .divsel().clear_bit() // interpret divisor as DIV, not 2**(DIV+1)
        .div().variant(1) // divide by 1 (= no division)
        .runstdby().set_bit() // run even in standby
        .idc().clear_bit() // no need to improve duty cycle; we are not dividing
        .oe().clear_bit() // no explicit I/O output
        .src().xosc32k() // take time from XOSC32K
    );
    while peripherals.GCLK.syncbusy.read().genctrl3().bit_is_set() {
    }

    // connect GCG3 as slow clock to SERCOM0
    const GCLK_SERCOM0_TO_SERCOM4_SLOW: usize = 18;
    peripherals.GCLK.pchctrl[GCLK_SERCOM0_TO_SERCOM4_SLOW].modify(|_, w| w
        .gen().gclk3() // take from GCG3
        .chen().set_bit() // enable
    );
}


/// Performs microcontroller initialization.
pub(crate) fn initialize_microcontroller(peripherals: &mut Peripherals) {
    // we want to switch to performance level 2 (PL2) as soon as possible;
    // there isn't much documentation on flash wait states in the datasheet,
    // but a wait state count of 2 has been listed in the datasheet for 3.3V and PL2
    // (encoded in the SVD as "DUAL")
    peripherals.NVMCTRL.ctrlb.modify(|_, w| w
        .rws().dual()
    );

    // switch to PL2
    peripherals.PM.plcfg.modify(|_, w| w
        .plsel().pl2()
    );
    while peripherals.PM.intflag.read().plrdy().bit_is_clear() {
    }

    setup_clocks(peripherals);
}
