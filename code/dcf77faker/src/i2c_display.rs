use core::time::Duration;

use atsaml21g18b::Peripherals;

use crate::i2c_controller::{I2cError, Sercom0I2cController, SercomI2cController};
use crate::tick::delay;


const LONG_DELAY: Duration = Duration::from_micros(2_160);
const SHORT_DELAY: Duration = Duration::from_nanos(52_600);


/// Common trait for I2C character-based liquid crystal displays consisting of:
///
/// * PCF8574 I2C-to-GPIO chip
/// * HD44780 LCD controller
///
/// The following PCF8574-to-HD44780 pinout is assumed:
///
/// | PCF8574 | HD44780     |
/// | ------- | ----------- |
/// | P7      | D7          |
/// | P6      | D6          |
/// | P5      | D5          |
/// | P4      | D4          |
/// | P3      | (backlight) |
/// | P2      | E           |
/// | P1      | R/~W        |
/// | P0      | RS          |
pub(crate) trait I2cDisplay<T: SercomI2cController> {
    /// Obtains the address of the display on the I2C bus.
    fn display_address(&self) -> u8;

    /// Whether the user wants the backlight of the display turned on.
    fn wants_backlight(&self) -> bool;

    /// Changes whether the user wants the backlight of the display turned on.
    fn set_wants_backlight(&mut self, wants_backlight: bool);

    /// Transmits a nibble (4 bits) of data.
    fn transmit_nibble(&self, peripherals: &mut Peripherals, nibble: u8, rs: bool) -> Result<(), I2cError> {
        // pin mapping (bits 7 to 0):
        // D7, D6, D5, D4, BL, E, RW, RS
        // BL = backlight
        // E = "read the data now" (we pulse this for a bit)
        // RW = Read=1, Write=0 (always 0 for transmissions)
        // RS = Register Select (0 for command, 1 for data)

        // prepare the byte to transmit, with E low
        let backlight_flag = if self.wants_backlight() { 0b0000_1000 } else { 0b0000_0000 };
        let rs_flag = if rs { 0b0000_0001 } else { 0b0000_0000 };
        let mut transmit_me = (nibble << 4) | backlight_flag | rs_flag;

        // send (with E low)
        T::send(peripherals, self.display_address(), [transmit_me])?;
        delay(Duration::from_nanos(500));

        // pull E high
        transmit_me |= 0b0000_0100;

        // send (with E high)
        T::send(peripherals, self.display_address(), [transmit_me])?;
        delay(Duration::from_nanos(500));

        // pull E low
        transmit_me &= 0b1111_1011;

        // send (with E low)
        T::send(peripherals, self.display_address(), [transmit_me])?;
        delay(Duration::from_nanos(500));

        Ok(())
    }

    /// Transmits a byte (8 bits) of data.
    fn transmit_byte(&self, peripherals: &mut Peripherals, byte: u8, rs: bool) -> Result<(), I2cError> {
        // in 4-bit mode, the upper nibble is transmitted first

        // transmit the upper nibble
        let upper_error = self.transmit_nibble(peripherals, byte >> 4, rs);

        // transmit the lower nibble
        let lower_error = self.transmit_nibble(peripherals, byte & 0xF, rs);

        upper_error.or(lower_error)
    }

    /// Waits for the "short delay" (nominally 37µs according to the HD44780 datasheet).
    fn short_delay() {
        delay(SHORT_DELAY);
    }

    /// Waits for the "long delay" (nominally 1.52ms according to the HD44780 datasheet).
    fn long_delay() {
        delay(LONG_DELAY);
    }

    /// Updates the backlight status for the display.
    fn update_backlight(&self, peripherals: &mut Peripherals) -> Result<(), I2cError> {
        // as long as we keep E low, the display controller ignores us
        // => simply transmit all low bits except for the backlight
        let backlight_byte = if self.wants_backlight() { 0b0000_1000 } else { 0b0000_0000 };
        T::send(peripherals, self.display_address(), [backlight_byte])
    }

    /// Perform basic display setup.
    fn basic_setup(&self, peripherals: &mut Peripherals) -> Result<(), I2cError> {
        // set display to 8-bit mode
        // send the same nibble three times so that we take care of all situations:
        // * 8-bit mode (reads 0011_0000, sets to 8 bit)
        // * 4-bit mode, start of a byte (reads 0011 & 0011, sets to 8 bit, reads 0011_0000, sets to 8 bit)
        // * 4-bit mode, middle of a byte (reads 0011, executes something, then reads 0011 & 0011, sets to 8 bit)
        self.transmit_nibble(peripherals, 0b0011, false)?;
        Self::long_delay();
        self.transmit_nibble(peripherals, 0b0011, false)?;
        Self::short_delay();
        self.transmit_nibble(peripherals, 0b0011, false)?;
        Self::short_delay();

        // set display to 4-bit mode
        self.transmit_nibble(peripherals, 0b0010, false)?;
        Self::short_delay();
        self.transmit_byte(peripherals, 0b0010_1000, false)?;
        Self::short_delay();

        // disable display
        self.transmit_byte(peripherals, 0b0000_1000, false)?;
        Self::short_delay();

        // clear display and go home
        self.transmit_byte(peripherals, 0b0000_0001, false)?;
        Self::long_delay();

        // increment but don't shift
        self.transmit_byte(peripherals, 0b0000_0110, false)?;
        Self::short_delay();

        // enable display
        self.transmit_byte(peripherals, 0b0000_1100, false)?;
        Self::short_delay();

        Ok(())
    }

    /// Move to a different location on the display.
    fn set_location(&self, peripherals: &mut Peripherals, location: u8) -> Result<(), I2cError> {
        self.transmit_byte(peripherals, 0b1000_0000 | location, false)
    }

    /// Write text at the current location on the display.
    fn write_text<I: IntoIterator<Item = u8>>(&self, peripherals: &mut Peripherals, text: I) -> Result<(), I2cError> {
        for b in text {
            self.transmit_byte(peripherals, b, true)?;
            Self::short_delay();
        }
        Ok(())
    }
}


/// I2C LCD on Two-Wire Interface 0.
pub struct I2cDisplaySercom0 {
    display_address: u8,
    wants_backlight: bool,
}
impl I2cDisplaySercom0 {
    pub const fn new(
        display_address: u8,
        wants_backlight: bool,
    ) -> Self {
        Self {
            display_address,
            wants_backlight,
        }
    }
}
impl I2cDisplay<Sercom0I2cController> for I2cDisplaySercom0 {
    #[inline] fn display_address(&self) -> u8 { self.display_address }
    #[inline] fn wants_backlight(&self) -> bool { self.wants_backlight }
    #[inline] fn set_wants_backlight(&mut self, wants_backlight: bool) { self.wants_backlight = wants_backlight; }
}
