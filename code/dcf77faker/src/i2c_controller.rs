//! Code to act as an I<sup>2</sup>C controller.
//!
//! Controllers were previously known as "masters".


use core::fmt;

use atsaml21g18b::Peripherals;
use atsaml21g18b::sercom0::I2CM;

use crate::init::CORE_CLOCK_SPEED_HZ;


/// I<sup>2</sup>C speed in bits per second (SERCOM considers this equivalent to Hz).
const I2C_SPEED_HZ: u32 = 100_000;


const CMD_REPEATED_START: u8 = 0x1;
const CMD_BYTE_READ: u8 = 0x2;
const CMD_STOP: u8 = 0x3;


const fn calculate_baud_divisor() -> u8 {
    // f_SCL = f_GCLK / (10 + 2*BAUD + f_GCLK * T_RISE)
    // datasheet table 46-12 mentions worst-case T_RISE = 13 ns = 13/1_000_000_000 s

    // I2C_SPEED_HZ = CORE_CLOCK_SPEED_HZ / (10 + 2*BAUD + CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s)
    // I2C_SPEED_HZ * (10 + 2*BAUD + CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s) = CORE_CLOCK_SPEED_HZ
    // 10 + 2*BAUD + CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s = CORE_CLOCK_SPEED_HZ / I2C_SPEED_HZ
    // 10 + 2*BAUD = CORE_CLOCK_SPEED_HZ / I2C_SPEED_HZ - CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s
    // 2*BAUD = CORE_CLOCK_SPEED_HZ / I2C_SPEED_HZ - CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s - 10
    // BAUD = (CORE_CLOCK_SPEED_HZ / I2C_SPEED_HZ - CORE_CLOCK_SPEED_HZ * 13/1_000_000_000 s - 10) / 2

    ((CORE_CLOCK_SPEED_HZ / I2C_SPEED_HZ - CORE_CLOCK_SPEED_HZ * 13 / 1_000_000_000 - 10) / 2) as u8
}


/// The type of error that may occur during an I<sup>2</sup>C operation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum I2cErrorKind {
    /// The I<sup>2</sup>C controller has been preempted by another device becoming controller.
    ArbitrationLost,

    /// An error happened on the I<sup>2</sup>C bus.
    BusError,

    /// A packet was not acknowledged by the other device.
    ///
    /// `index` specifies at which location in the data no acknowledgement was provided. If the
    /// unacknowledged byte was the address byte, `index` is equal to [`usize::MAX`].
    NotAcknowledged,

    /// The given address is not a valid address.
    ///
    /// This error is generally raised if the topmost bit is set.
    InvalidAddress,
}
impl I2cErrorKind {
    pub const fn to_error(&self, byte_info: I2cErrorByteInfo) -> I2cError {
        I2cError {
            kind: *self,
            byte_info,
        }
    }

    pub const fn at_address(&self, address: u8) -> I2cError {
        self.to_error(I2cErrorByteInfo::Address(address))
    }

    pub const fn at_data_index(&self, data: u8, index: usize) -> I2cError {
        self.to_error(I2cErrorByteInfo::Data {
            index,
            byte: data,
        })
    }

    pub fn at_stop_bit(&self) -> I2cError {
        self.to_error(I2cErrorByteInfo::StopBit)
    }
}
impl fmt::Display for I2cErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArbitrationLost
                => write!(f, "bus arbitration lost"),
            Self::BusError
                => write!(f, "bus error"),
            Self::NotAcknowledged
                => write!(f, "byte not acknowledged"),
            Self::InvalidAddress
                => write!(f, "invalid address"),
        }
    }
}


/// The byte of an I<sup>2</sup>C transmission at which an error was detected.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum I2cErrorByteInfo {
    /// The address byte (includes the read/write flag).
    Address(u8),

    /// The data byte at the given index.
    Data { index: usize, byte: u8 },

    /// The stop bit at the end of the transmission.
    StopBit,
}
impl I2cErrorByteInfo {
    /// Whether the byte position is the address.
    pub fn is_address(&self) -> bool {
        match self {
            Self::Address(_) => true,
            _ => false,
        }
    }
}
impl fmt::Display for I2cErrorByteInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(address)
                => write!(f, "address byte 0b{:07b}", address),
            Self::Data { index, byte }
                => write!(f, "data byte {0} (0x{0:02X}) at index {1} (0x{1:X})", byte, index),
            Self::StopBit
                => write!(f, "stop bit"),
        }
    }
}


/// An error that may occur during an I<sup>2</sup>C operation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct I2cError {
    /// The kind of error that occurred.
    pub kind: I2cErrorKind,

    /// Information about the byte being transferred when the error occurred.
    pub byte_info: I2cErrorByteInfo,
}
impl fmt::Display for I2cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.byte_info)
    }
}


/// A SERCOM device that can act as an I<sup>2</sup>C controller.
pub(crate) trait SercomI2cController {
    /// Obtains a pointer to the SERCOM register block.
    fn get_register_block(peripherals: &mut Peripherals) -> &atsaml21g18b::sercom0::I2CM;

    /// Sets up the SERCOM device as an I<sup>2</sup>C controller.
    fn setup_controller(peripherals: &mut Peripherals) {
        let register_block = Self::get_register_block(peripherals);

        // reset SERCOM
        register_block.ctrla.modify(|_, w| w
            .swrst().set_bit()
        );
        while register_block.ctrla.read().swrst().bit_is_set() || register_block.syncbusy.read().swrst().bit_is_set() {
        }

        // basic configuration
        register_block.ctrla.modify(|_, w| w
            .mode().variant(0x5) // I2C controller
            .pinout().clear_bit() // disable 4-bit mode
            .sdahold().variant(0) // no SDA hold time relative to the negative edge
            .mexttoen().clear_bit() // no controller SCL-low-extend timeout
            .sexttoen().clear_bit() // no peripheral SCL-low-extend timeout
            .speed().variant(0) // standard speed (100 kHz)
            .sclsm().clear_bit() // regular SCL clock-stretch mode
            .lowtouten().clear_bit() // no SCL-low timeout
        );
        register_block.ctrlb.modify(|_, w| w
            .smen().clear_bit() // no smart mode
            .qcen().clear_bit() // no quick command
        );
        register_block.baud.modify(|_, w| w
            .baud().variant(calculate_baud_divisor())
            .baudlow().variant(0) // use BAUD for BAUDLOW
        );

        // enable I2C controller
        register_block.ctrla.modify(|_, w| w
            .enable().set_bit()
        );
        while register_block.syncbusy.read().enable().bit_is_set() {
        }

        // grab the bus
        register_block.status.modify(|_, w| w
            .busstate().variant(0b01)
        );
        while register_block.syncbusy.read().sysop().bit_is_set() {
        }
    }

    /// Waits until a byte is transmitted, then checks the current bus status and returns the
    /// corresponding error if one has occurred.
    fn wait_and_check_bus_status(register_block: &I2CM, byte_info: I2cErrorByteInfo) -> Result<(), I2cError> {
        // wait until our controller status is known, then clear that bit
        while register_block.intflag.read().mb().bit_is_clear() {
        }
        unsafe {
            register_block.intflag.write_with_zero(|w| w
                .mb().set_bit()
            )
        };

        let bus_status = register_block.status.read();
        // everything OK = MB
        // arbitration lost = MB | ARBLOST
        // bus error = MB | ARBLOST | BUSERR
        // (but MB is no longer set)
        if bus_status.buserr().bit_is_set() {
            unsafe {
                register_block.status.write_with_zero(|w| w
                    .buserr().set_bit()
                    .arblost().set_bit()
                )
            };
            return Err(I2cErrorKind::BusError.to_error(byte_info));
        }
        if bus_status.arblost().bit_is_set() {
            unsafe {
                register_block.status.write_with_zero(|w| w
                    .arblost().set_bit()
                )
            };
            return Err(I2cErrorKind::ArbitrationLost.to_error(byte_info));
        }

        // maybe the transmission succeeded but nobody responded
        if bus_status.rxnack().bit_is_clear() {
            return Err(I2cErrorKind::NotAcknowledged.to_error(byte_info));
        }

        Ok(())
    }

    /// Sends data to a peripheral device.
    fn send<I: IntoIterator<Item = u8>>(peripherals: &mut Peripherals, address: u8, data: I) -> Result<(), I2cError> {
        if address & 0b1000_0000 != 0 {
            return Err(I2cErrorKind::InvalidAddress.at_address(address));
        }

        let register_block = Self::get_register_block(peripherals);

        // set address
        let address_and_write: u8 = address << 1;
        register_block.addr.modify(|_, w| w
            .addr().variant(address_and_write.into())
            .lenen().clear_bit() // no DMA
            .hs().clear_bit() // no high-speed transfer
            .tenbiten().clear_bit() // disable 10-bit addressing
        );
        while register_block.syncbusy.read().sysop().bit_is_set() {
        }

        Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::Address(address))?;

        // write data
        let mut bytes_written = 0;
        for byte in data {
            // send
            register_block.data.modify(|_, w| w
                .data().variant(byte)
            );
            while register_block.syncbusy.read().sysop().bit_is_set() {
            }
            Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::Data { index: bytes_written, byte })?;
            bytes_written += 1;
        }

        // send STOP
        register_block.ctrlb.modify(|_, w| w
            .cmd().variant(CMD_STOP)
        );
        while register_block.syncbusy.read().sysop().bit_is_set() {
        }
        Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::StopBit)
    }

    /// Receives data from a peripheral device.
    fn receive<F: FnMut(u8) -> bool>(peripherals: &mut Peripherals, address: u8, mut handle_byte: F) -> Result<(), I2cError> {
        if address & 0b1000_0000 != 0 {
            return Err(I2cErrorKind::InvalidAddress.at_address(address));
        }

        let register_block = Self::get_register_block(peripherals);

        // set address
        let address_and_read: u8 = (address << 1) | 0b1;
        register_block.addr.modify(|_, w| w
            .addr().variant(address_and_read.into())
            .lenen().clear_bit() // no DMA
            .hs().clear_bit() // no high-speed transfer
            .tenbiten().clear_bit() // disable 10-bit addressing
        );
        while register_block.syncbusy.read().sysop().bit_is_set() {
        }
        Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::Address(address))?;

        // read data
        let mut bytes_read = 0;
        loop {
            // receive
            let byte = register_block.data.read().data().bits();
            Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::Data { byte, index: bytes_read })?;
            bytes_read += 1;

            let acknowledge = handle_byte(byte);
            if acknowledge {
                // send acknowledge bit and read again
                register_block.ctrlb.modify(|_, w| w
                    .ackact().set_bit()
                    .cmd().variant(CMD_BYTE_READ)
                );
                while register_block.syncbusy.read().sysop().bit_is_set() {
                }
            } else {
                // don't acknowledge and send STOP
                register_block.ctrlb.modify(|_, w| w
                    .ackact().clear_bit()
                    .cmd().variant(CMD_STOP)
                );
                while register_block.syncbusy.read().sysop().bit_is_set() {
                }
                break;
            }
        }
        Self::wait_and_check_bus_status(register_block, I2cErrorByteInfo::StopBit)
    }
}


pub(crate) struct Sercom0I2cController;
impl SercomI2cController for Sercom0I2cController {
    fn get_register_block(peripherals: &mut Peripherals) -> &atsaml21g18b::sercom0::I2CM {
        unsafe { (&*atsaml21g18b::SERCOM0::PTR).i2cm() }
    }
}

pub(crate) struct Sercom1I2cController;
impl SercomI2cController for Sercom1I2cController {
    fn get_register_block(peripherals: &mut Peripherals) -> &atsaml21g18b::sercom0::I2CM {
        unsafe { (&*atsaml21g18b::SERCOM1::PTR).i2cm() }
    }
}
