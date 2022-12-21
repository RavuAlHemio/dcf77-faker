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

use atsaml21g18b::{CorePeripherals, interrupt, Peripherals};
use cortex_m_rt::entry;

use crate::dcf77::Dcf77Data;
use crate::i2c_controller::{Sercom0I2cController, SercomI2cController};
use crate::i2c_display::{I2cDisplay, I2cDisplaySercom0};
use crate::init::CORE_CLOCK_SPEED_HZ;
use crate::pin::PeripheralIndex;
use crate::pwm::{Tcc0Pwm, TccPwm};
use crate::sync_vcell::SyncVolatileCell;


static SECOND: SyncVolatileCell<u8> = SyncVolatileCell::new(59);
static DCF77_DATA: SyncVolatileCell<Dcf77Data> = SyncVolatileCell::new(Dcf77Data::new());
static UPDATE_TIME: SyncVolatileCell<bool> = SyncVolatileCell::new(false);


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
    let mut core_peripherals = CorePeripherals::take()
        .expect("core peripherals already taken?!");
    let mut peripherals = Peripherals::take()
        .expect("peripherals already taken?!");

    crate::init::initialize_microcontroller(&mut peripherals);
    crate::tick::enable_tick_clock(&mut core_peripherals);

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

    // set up I2C
    Sercom0I2cController::setup_controller(&mut peripherals);

    // set up display
    let i2c_display = I2cDisplaySercom0::new(0b010_0111, true);
    let _ = i2c_display.basic_setup(&mut peripherals);
    let _ = i2c_display.set_location(&mut peripherals, 0);
    let _ = i2c_display.write_text(&mut peripherals, *b"DCF77 Faker");

    // set up PWM
    Tcc0Pwm::setup_pwm(&mut peripherals);
    Tcc0Pwm::set_period_and_duty_cycle(
        &mut peripherals,
        CORE_CLOCK_SPEED_HZ / dcf77::FREQUENCY_HZ,
        0,
    );
    Tcc0Pwm::start_generation(&mut peripherals);

    loop {
        while !UPDATE_TIME.get() {
        }

        UPDATE_TIME.set(false);
        let second = SECOND.get();

        // send over the new time
        let mut time_info: [u8; 17] = *b"xx.xx.xx xx:xx:xx";
        let data = DCF77_DATA.get();
        time_info[0] = b'0' + data.day_of_month_tens;
        time_info[1] = b'0' + data.day_of_month_ones;
        time_info[3] = if data.month_ten { b'1' } else { b'0' };
        time_info[4] = b'0' + data.month_ones;
        time_info[6] = b'0' + data.year_in_century_tens;
        time_info[7] = b'0' + data.year_in_century_ones;
        time_info[9] = b'0' + data.hour_tens;
        time_info[10] = b'0' + data.hour_ones;
        time_info[12] = b'0' + data.minute_tens;
        time_info[13] = b'0' + data.minute_ones;
        time_info[15] = b'0' + (second / 10);
        time_info[16] = b'0' + (second % 10);

        let _ = i2c_display.set_location(&mut peripherals, 20);
        let _ = i2c_display.write_text(&mut peripherals, time_info);
    }
}


#[interrupt]
fn RTC() {
    // fired 32x per second
    static mut COUNTER: u8 = 31;
    static mut MINUTE: u64 = 0;

    let mut peripherals = unsafe { Peripherals::steal() };

    // increment counter
    *COUNTER = (*COUNTER + 1) % 32;
    if *COUNTER != 0 {
        return;
    }

    // increment second
    let mut second = SECOND.get() + 1;
    if second == 60 {
        second = 0;
    }
    SECOND.set(second);
    if second == 59 {
        // turn off modulation
        Tcc0Pwm::set_duty_cycle(&mut peripherals, 0);

        // calculate a new minute
        let mut dcf77_data = DCF77_DATA.get();
        dcf77_data.increment_minute();
        DCF77_DATA.set(dcf77_data);
        *MINUTE = dcf77_data.to_bits();
    } else {
        // regular behavior

        // lop the last bit off of the minute
        let long_duty_cycle = (*MINUTE & 0b1) != 0;
        *MINUTE >>= 1;

        let period = init::CORE_CLOCK_SPEED_HZ / dcf77::FREQUENCY_HZ;
        if long_duty_cycle {
            Tcc0Pwm::set_duty_cycle(&mut peripherals, period / 2);
        } else {
            Tcc0Pwm::set_duty_cycle(&mut peripherals, period / 44);
        }
    }

    // update time on the display
    UPDATE_TIME.set(true);
}
