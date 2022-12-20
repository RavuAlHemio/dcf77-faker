//! Code relevant to pulse-width modulation.

use atsaml21g18b::Peripherals;


/// PWM functionality implemented using a TCC module.
pub(crate) trait TccPwm {
    /// Unmasks the clock signals going to the TCC device.
    fn enable_clock(peripherals: &mut Peripherals);

    /// Obtains a pointer to the TCC register block.
    fn get_register_block(peripherals: &mut Peripherals) -> &atsaml21g18b::tcc0::RegisterBlock;

    /// Sets up PWM.
    ///
    /// PWM is set up in normal mode with regular polarity. This has the following behavior:
    ///
    /// 1. The counter counts up (the counter value is incremented with every counter trigger).
    ///
    /// 2. When the counter equals zero, the output is set high.
    ///
    /// 3. When the counter reaches `CC0`, the output is set low.
    ///
    /// 4. When the counter reaches `PER`, it is reset to 0.
    ///
    /// The values for `CC0` ([`set_duty_cycle`](TccPwm::set_duty_cycle)) and `PER`
    /// ([`set_period`](TccPwm::set_period)) are not set by this function and must be set by the user.
    fn setup_pwm(peripherals: &mut Peripherals) {
        Self::enable_clock(peripherals);

        let register_block = Self::get_register_block(peripherals);

        // reset TCC
        register_block.ctrla.modify(|_, w| w
            .swrst().set_bit()
        );
        while register_block.ctrla.read().swrst().bit_is_set() || register_block.syncbusy.read().swrst().bit_is_set() {
        }

        // basic setup
        register_block.ctrla.modify(|_, w| w
            .cpten0().clear_bit() // compare, don't capture on channel 0
            .cpten1().clear_bit() // compare, don't capture on channel 1
            .cpten2().clear_bit() // compare, don't capture on channel 2
            .cpten3().clear_bit() // compare, don't capture on channel 3
            .dmaos().clear_bit() // no DMA one-shot trigger
            .msync().clear_bit() // no master synchronization
            .alock().clear_bit() // no auto-lock (= no CTRLB.LUPD changes on overflow/underflow/retrigger)
            .prescsync().presc() // reload/reset counter on tick of prescaled clock
            .runstdby().set_bit() // run TCC0 in standby
            .prescaler().div1() // no prescaling (divide by 1)
            .resolution().none() // no dithering
        );
        loop {
            let syncbusy = register_block.syncbusy.read();
            let stop_waiting =
                syncbusy.cc0().bit_is_clear()
                && syncbusy.cc1().bit_is_clear()
                && syncbusy.cc2().bit_is_clear()
                && syncbusy.cc3().bit_is_clear()
            ;
            if stop_waiting {
                break;
            }
        }

        register_block.ctrlbclr.modify(|_, w| w
            // note that when we set a bit to 1 here, it is set to 0 in the underlying register
            // (this is general behavior of the CTRLBCLR register)
            .dir().set_bit() // count upward
            .lupd().set_bit() // no update locking
            .oneshot().set_bit() // no one-shot
            .idxcmd().set() // we don't use RAMP2/RAMP2A anyway; loading anything with a 1 bit clears this whole field
            .cmd().retrigger() // loading anything with 1 bit clears this whole field
        );
        while register_block.syncbusy.read().ctrlb().bit_is_set() {
        }

        register_block.wave.modify(|_, w| w
            .wavegen().npwm() // normal PWM mode
            .ramp().ramp1() // regular ramp operation
            .ciperen().clear_bit() // no circular buffer on period
            .ciccen0().clear_bit() // no circular buffer on compare channel 0
            .ciccen1().clear_bit() // no circular buffer on compare channel 1
            .ciccen2().clear_bit() // no circular buffer on compare channel 2
            .ciccen3().clear_bit() // no circular buffer on compare channel 3
            .pol0().clear_bit() // regular polarity on output channel 0
            .pol1().clear_bit() // regular polarity on output channel 1
            .pol2().clear_bit() // regular polarity on output channel 2
            .pol3().clear_bit() // regular polarity on output channel 3
            .swap0().clear_bit() // no swap on dead-time insertion on output 0
            .swap1().clear_bit() // no swap on dead-time insertion on output 1
            .swap2().clear_bit() // no swap on dead-time insertion on output 2
            .swap3().clear_bit() // no swap on dead-time insertion on output 3
        );
        while register_block.syncbusy.read().wave().bit_is_set() {
        }

        // no waveform extension weirdness, just send the outputs to the pins
        register_block.wexctrl.modify(|_, w| w
            .otmx().variant(0) // 1:1 output-to-pin mapping
            .dtien0().clear_bit() // no dead-time insertion on pin 0
            .dtien1().clear_bit() // no dead-time insertion on pin 1
            .dtien2().clear_bit() // no dead-time insertion on pin 2
            .dtien3().clear_bit() // no dead-time insertion on pin 3
            .dtls().variant(0) // no dead-time low side outputs value
            .dths().variant(0) // no dead-time high side outputs value
        );
    }

    /// Sets the period of the PWM generation.
    ///
    /// The TCC increases the counter on every cycle of the core clock ([`CORE_CLOCK_SPEED_HZ`]).
    /// The period value defines how many times this has to occur before the counter is reset to 0.
    /// This defines the frequency of the PWM signal; to define the duty cycle, see
    /// [`set_duty_cycle`].
    ///
    /// [`CORE_CLOCK_SPEED_HZ`](crate::init::CORE_CLOCK_SPEED_HZ)
    /// [`set_duty_cycle`](TccPwm::set_duty_cycle)
    fn set_period(peripherals: &mut Peripherals, period: u32) {
        let register_block = Self::get_register_block(peripherals);
        register_block.per().write(|w| w
            .per().variant(period)
        );
        while register_block.syncbusy.read().per().bit_is_set() {
        }
    }

    /// Sets the duty cycle of the PWM generation.
    ///
    /// The TCC increases the counter on every cycle of the core clock ([`CORE_CLOCK_SPEED_HZ`]).
    /// The duty cycle value defines how many times this has to occur before the output signal,
    /// enabled when the counter is 0, is disabled again. It is then re-enabled when the counter
    /// reaches the period value (see [`set_period`]), as it is then reset to 0.
    ///
    /// [`CORE_CLOCK_SPEED_HZ`](crate::init::CORE_CLOCK_SPEED_HZ)
    /// [`set_period`](TccPwm::set_period)
    fn set_duty_cycle(peripherals: &mut Peripherals, duty_cycle: u32) {
        let register_block = Self::get_register_block(peripherals);
        register_block.cc()[0].write(|w| w
            .cc().variant(duty_cycle)
        );
        while register_block.syncbusy.read().cc0().bit_is_set() {
        }
    }

    /// Sets the period and duty cycle of the PWM generation.
    ///
    /// This is equivalent to calling [`set_period`] and [`set_duty_cycle`] separately, but it sets
    /// both values right after one another before waiting for them to synchronize, which might be
    /// faster if both values are changed simultaneously.
    ///
    /// [`set_period`](TccPwm::set_period)
    /// [`set_duty_cycle`](TccPwm::set_duty_cycle)
    fn set_period_and_duty_cycle(peripherals: &mut Peripherals, period: u32, duty_cycle: u32) {
        let register_block = Self::get_register_block(peripherals);
        register_block.per().write(|w| w
            .per().variant(period)
        );
        register_block.cc()[0].write(|w| w
            .cc().variant(duty_cycle)
        );
        loop {
            let syncbusy = register_block.syncbusy.read();
            let done =
                syncbusy.per().bit_is_clear()
                && syncbusy.cc0().bit_is_clear()
            ;
            if done {
                break;
            }
        }
    }

    /// Starts the timer.
    fn start_generation(peripherals: &mut Peripherals) {
        let register_block = Self::get_register_block(peripherals);
        register_block.ctrla.modify(|_, w| w
            .enable().set_bit()
        );
        while register_block.syncbusy.read().enable().bit_is_set() {
        }
    }

    /// Stops the timer.
    fn stop_generation(peripherals: &mut Peripherals) {
        let register_block = Self::get_register_block(peripherals);
        register_block.ctrla.modify(|_, w| w
            .enable().clear_bit()
        );
        while register_block.syncbusy.read().enable().bit_is_set() {
        }
    }
}

pub(crate) struct Tcc0Pwm;
impl TccPwm for Tcc0Pwm {
    fn enable_clock(peripherals: &mut Peripherals) {
        const GCLK_TCC0_THROUGH_TCC1: usize = 25;

        peripherals.MCLK.apbcmask.modify(|_, w| w
            .tcc0_().set_bit()
        );
        peripherals.GCLK.pchctrl[GCLK_TCC0_THROUGH_TCC1].modify(|_, w| w
            .chen().set_bit()
        );
    }

    fn get_register_block(peripherals: &mut Peripherals) -> &atsaml21g18b::tcc0::RegisterBlock {
        unsafe { &*atsaml21g18b::TCC0::PTR }
    }
}
