//! The DCF77 time transmission protocol.


pub const FREQUENCY_HZ: u32 = 77_500;


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Dcf77Data {
    // start of minute (bit :00) is always 0

    /// Civil warning bits. (bits :01 through :14)
    ///
    /// Only the bottom 14 bits of this value are used.
    pub civil_warning: u16,

    /// Abnormal transmitter operation. (bit :15)
    pub abnormal_operation: bool,

    /// Summer time announcement. (bit :16)
    ///
    /// Set during the hour before a summer <-> winter time changeover.
    pub summer_announcement: bool,

    /// Set if CEST (summer time) is in effect. (bit :17)
    pub cest: bool,

    /// Set if CET (winter time) is in effect. (bit :18)
    pub cet: bool,

    /// Leap second announcement. (bit :19)
    ///
    /// Set during the hour before the insertion of a leap second.
    pub leap_second_announcement: bool,

    // start of time (bit :20) is always 1

    /// Ones of the minute. (bits :21 through :24)
    ///
    /// The bits represent the values 1, 2, 4 and 8, in that order.
    pub minute_ones: u8,

    /// Tens of the minute. (bits :25 through :27)
    ///
    /// The bits represent the values 10, 20 and 40, in that order.
    pub minute_tens: u8,

    // bit :28 is even parity over minute bits

    /// Ones of the hour. (bits :29 through :32)
    ///
    /// The bits represent the values 1, 2, 4 and 8, in that order.
    pub hour_ones: u8,

    /// Tens of the hour. (bits :33 through :34)
    ///
    /// The bits represent the values 10 and 20 in that order.
    pub hour_tens: u8,

    // bit :35 is even parity over hour bits

    /// Ones of the day of month. (bits :36 through :39)
    ///
    /// The bits represent the values 1, 2, 4, and 8, in that order.
    pub day_of_month_ones: u8,

    /// Tens of the day of month. (bits :40 through :41)
    ///
    /// The bits represent the values 10 and 20, in that order.
    pub day_of_month_tens: u8,

    /// Day of week. (bits :42 through :44)
    ///
    /// The bits represent the values 1, 2 and 4, in that order. Valid values are from 1 (Monday) to
    /// 7 (Sunday).
    pub day_of_week: u8,

    /// Ones of the month. (bits :45 through :48)
    ///
    /// The bits represent the values 1, 2, 4 and 8, in that order.
    pub month_ones: u8,

    /// Tens of the month. (bit :49)
    ///
    /// The bit represents the value 10.
    pub month_ten: bool,

    /// Ones of the year within its century. (bits :50 through :53)
    ///
    /// The bits represent the values 1, 2, 4 and 8, in that order.
    pub year_in_century_ones: u8,

    /// Tens of the year within its century. (bits :54 through :57)
    ///
    /// The bits represent the values 10, 20, 40 and 80, in that order.
    pub year_in_century_tens: u8,

    // bit :58 is even parity over date bits :36 through :57

    // on bit :59, modulation is fully disabled
}
impl Dcf77Data {
    pub const fn new() -> Self {
        Self {
            civil_warning: 0,
            abnormal_operation: false,
            summer_announcement: false,
            cest: true,
            cet: false,
            leap_second_announcement: false,
            minute_ones: 0,
            minute_tens: 4,
            hour_tens: 1,
            hour_ones: 0,
            day_of_month_tens: 1,
            day_of_month_ones: 0,
            day_of_week: 2,
            month_ones: 4,
            month_ten: false,
            year_in_century_ones: 0,
            year_in_century_tens: 9,
        }
    }

    pub fn increment_minute(&mut self) {
        self.minute_ones += 1;
        if self.minute_ones < 10 {
            return;
        }

        self.minute_ones = 0;
        self.minute_tens += 1;
        if self.minute_tens < 6 {
            return;
        }

        self.minute_tens = 0;
        self.hour_ones += 1;
        if self.hour_tens == 2 && self.hour_ones >= 4 {
            // don't bother incrementing the date
            self.hour_ones = 0;
            self.hour_tens = 0;
            return;
        } else if self.hour_ones < 10 {
            return;
        }

        self.hour_ones = 0;
        self.hour_tens += 1;

        // don't bother with the date
    }

    pub const fn to_bits(&self) -> u64 {
        let mut value = 0;

        // bit 0 is always 0

        // bits 1 through 14
        value |= ((self.civil_warning & 0b11_1111_1111_1111) as u64) << 1;

        // bit 15
        if self.abnormal_operation {
            value |= 1 << 15;
        }

        // bit 16
        if self.summer_announcement {
            value |= 1 << 16;
        }

        // bit 17
        if self.cest {
            value |= 1 << 17;
        }

        // bit 18
        if self.cet {
            value |= 1 << 18;
        }

        // bit 19
        if self.leap_second_announcement {
            value |= 1 << 19;
        }

        // bit 20
        value |= 1 << 20;

        // bits 21 through 27
        let mut minute_parity = false;
        if self.minute_ones & 1 != 0 {
            value |= 1 << 21;
            minute_parity = !minute_parity;
        }
        if self.minute_ones & 2 != 0 {
            value |= 1 << 22;
            minute_parity = !minute_parity;
        }
        if self.minute_ones & 4 != 0 {
            value |= 1 << 23;
            minute_parity = !minute_parity;
        }
        if self.minute_ones & 8 != 0 {
            value |= 1 << 24;
            minute_parity = !minute_parity;
        }
        if self.minute_tens & 1 != 0 {
            value |= 1 << 25;
            minute_parity = !minute_parity;
        }
        if self.minute_tens & 2 != 0 {
            value |= 1 << 26;
            minute_parity = !minute_parity;
        }
        if self.minute_tens & 4 != 0 {
            value |= 1 << 27;
            minute_parity = !minute_parity;
        }

        // bit 28
        if minute_parity {
            value |= 1 << 28;
        }

        // bits 29 thorugh 34
        let mut hour_parity = false;
        if self.hour_ones & 1 != 0 {
            value |= 1 << 29;
            hour_parity = !hour_parity;
        }
        if self.hour_ones & 2 != 0 {
            value |= 1 << 30;
            hour_parity = !hour_parity;
        }
        if self.hour_ones & 4 != 0 {
            value |= 1 << 31;
            hour_parity = !hour_parity;
        }
        if self.hour_ones & 8 != 0 {
            value |= 1 << 32;
            hour_parity = !hour_parity;
        }
        if self.hour_tens & 1 != 0 {
            value |= 1 << 33;
            hour_parity = !hour_parity;
        }
        if self.hour_tens & 2 != 0 {
            value |= 1 << 34;
            hour_parity = !hour_parity;
        }

        // bit 35
        if hour_parity {
            value |= 1 << 35;
        }

        // bits 36 through 41
        let mut date_parity = false;
        if self.day_of_month_ones & 1 != 0 {
            value |= 1 << 36;
            date_parity = !date_parity;
        }
        if self.day_of_month_ones & 2 != 0 {
            value |= 1 << 37;
            date_parity = !date_parity;
        }
        if self.day_of_month_ones & 4 != 0 {
            value |= 1 << 38;
            date_parity = !date_parity;
        }
        if self.day_of_month_ones & 8 != 0 {
            value |= 1 << 39;
            date_parity = !date_parity;
        }
        if self.day_of_month_tens & 1 != 0 {
            value |= 1 << 40;
            date_parity = !date_parity;
        }
        if self.day_of_month_tens & 2 != 0 {
            value |= 1 << 41;
            date_parity = !date_parity;
        }

        // bits 42 through 44
        if self.day_of_week & 1 != 0 {
            value |= 1 << 42;
            date_parity = !date_parity;
        }
        if self.day_of_week & 2 != 0 {
            value |= 1 << 43;
            date_parity = !date_parity;
        }
        if self.day_of_week & 4 != 0 {
            value |= 1 << 44;
            date_parity = !date_parity;
        }

        // bits 45 through 49
        if self.month_ones & 1 != 0 {
            value |= 1 << 45;
            date_parity = !date_parity;
        }
        if self.month_ones & 2 != 0 {
            value |= 1 << 46;
            date_parity = !date_parity;
        }
        if self.month_ones & 4 != 0 {
            value |= 1 << 47;
            date_parity = !date_parity;
        }
        if self.month_ones & 8 != 0 {
            value |= 1 << 48;
            date_parity = !date_parity;
        }
        if self.month_ten {
            value |= 1 << 49;
            date_parity = !date_parity;
        }

        // bits 50 through 57
        if self.year_in_century_ones & 1 != 0 {
            value |= 1 << 50;
            date_parity = !date_parity;
        }
        if self.year_in_century_ones & 2 != 0 {
            value |= 1 << 51;
            date_parity = !date_parity;
        }
        if self.year_in_century_ones & 4 != 0 {
            value |= 1 << 52;
            date_parity = !date_parity;
        }
        if self.year_in_century_ones & 8 != 0 {
            value |= 1 << 53;
            date_parity = !date_parity;
        }
        if self.year_in_century_tens & 1 != 0 {
            value |= 1 << 54;
            date_parity = !date_parity;
        }
        if self.year_in_century_tens & 2 != 0 {
            value |= 1 << 55;
            date_parity = !date_parity;
        }
        if self.year_in_century_tens & 4 != 0 {
            value |= 1 << 56;
            date_parity = !date_parity;
        }
        if self.year_in_century_tens & 8 != 0 {
            value |= 1 << 57;
            date_parity = !date_parity;
        }

        // bit 58
        if date_parity {
            value |= 1 << 58;
        }

        value
    }
}
