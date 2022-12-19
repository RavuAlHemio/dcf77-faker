//! Access to parallel I/O ports made easy.


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PeripheralIndex {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
}
impl PeripheralIndex {
    /// Converts this peripheral index to the representation in the configuration registers.
    #[inline]
    pub const fn to_nibble(&self) -> u8 {
        match self {
            Self::A => 0x0,
            Self::B => 0x1,
            Self::C => 0x2,
            Self::D => 0x3,
            Self::E => 0x4,
            Self::F => 0x5,
            Self::G => 0x6,
            Self::H => 0x7,
            Self::I => 0x8,
        }
    }
}


#[macro_export]
macro_rules! board_pin {
    (set_io, $peri:expr, $pinbank:ident, $firstpin:expr $(, $pinnum:expr)*) => {
        board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$firstpin].modify(|_, w| w
            .pmuxen().clear_bit()
        )
        $(
            ;
            board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$pinnum].modify(|_, w| w
                .pmuxen().clear_bit()
            )
        )*
    };
    (set_peripheral, $peri:expr, $pinbank:ident, $firstpin:expr $(, $pinnum:expr)*) => {
        board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$firstpin].modify(|_, w| w
            .pmuxen().set_bit()
        )
        $(
            ;
            board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$pinnum].modify(|_, w| w
                .pmuxen().set_bit()
            )
        )*
    };
    (select_peripheral, $peri:expr, $periindex:expr, $pinbank:ident, $firstpin:expr $(, $pinnum:expr)*) => {
        board_pin!(pinbank_to_mux_reg, $peri.PORT, $pinbank)[$firstpin / 2].modify(|_, w| {
            if $firstpin % 2 == 0 {
                unsafe { w.pmuxe().bits($periindex.to_nibble()) }
            } else {
                unsafe { w.pmuxo().bits($periindex.to_nibble()) }
            }
        })
        $(
            ;
            board_pin!(pinbank_to_mux_reg, $peri.PORT, $pinbank)[$pinnum / 2].modify(|_, w| {
                if $pinnum % 2 == 0 {
                    unsafe { w.pmuxe().bits($periindex.to_nibble()) }
                } else {
                    unsafe { w.pmuxo().bits($periindex.to_nibble()) }
                }
            })
        )*
    };
    (make_input, $peri:expr, $pinbank:ident $(, $pinnum:expr)+) => {
        board_pin!(pinbank_to_dirclr_reg, $peri.PORT, $pinbank).write(|w| w
            .dirclr().variant(board_pin!(@bitmasking, 0 $(, $pinnum)+)) // equivalent to .dir().clear_bit() but no read-modify-write
        )
    };
    (make_output, $peri:expr, $pinbank:ident $(, $pinnum:expr)+) => {
        board_pin!(pinbank_to_dirset_reg, $peri.PORT, $pinbank).write(|w| w
            .dirset().variant(board_pin!(@bitmasking, 0 $(, $pinnum)+)) // equivalent to .dir().set_bit() but no R-M-W
        )
    };
    (enable_pull, $peri:expr, $pinbank:ident, $firstpin:expr $(, $pinnum:expr)*) => {
        board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$firstpin].modify(|_, w| w
            .pullen().set_bit()
        )
        $(
            ;
            board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$pinnum].modify(|_, w| w
                .pullen().set_bit()
            )
        )*
    };
    (disable_pull, $peri:expr, $pinbank:ident, $firstpin:expr $(, $pinnum:expr)*) => {
        board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$firstpin].modify(|_, w| w
            .pullen().clear_bit()
        )
        $(
            ;
            board_pin!(pinbank_to_cfg_reg, $peri.PORT, $pinbank)[$pinnum].modify(|_, w| w
                .pullen().clear_bit()
            )
        )*
    };
    (read_pin, $peri:expr, $pinbank:ident, $pinnum:expr) => {
        (board_pin!(pinbank_to_in_reg, $peri.PORT, $pinbank).read().bits() & (1 << $pinnum)) != 0
    };
    (read_pins, $peri:expr, $pinbank:ident) => {
        board_pin!(pinbank_to_in_reg, $peri.PORT, $pinbank).read().bits()
    };
    (set_high, $peri:expr, $pinbank:ident $(, $pinnum:expr)+) => {
        board_pin!(pinbank_to_outset_reg, $peri.PORT, $pinbank).write(|w| w
            .outset().variant(board_pin!(@bitmasking, 0 $(, $pinnum)+)) // equivalent to .out().set_bit() but no R-M-W
        )
    };
    (set_low, $peri:expr, $pinbank:ident $(, $pinnum:expr)+) => {
        board_pin!(pinbank_to_outclr_reg, $peri.PORT, $pinbank).write(|w| w
            .outclr().variant(board_pin!(@bitmasking, 0 $(, $pinnum)+)) // equivalent to .out().clear_bit() but no R-M-W
        )
    };

    (pinbank_to_cfg_reg, $port:expr, PA) => ($port.pincfg0_);
    (pinbank_to_cfg_reg, $port:expr, PB) => ($port.pincfg1_);
    (pinbank_to_mux_reg, $port:expr, PA) => ($port.pmux0_);
    (pinbank_to_mux_reg, $port:expr, PB) => ($port.pmux1_);
    (pinbank_to_in_reg, $port:expr, PA) => ($port.in0);
    (pinbank_to_in_reg, $port:expr, PB) => ($port.in1);
    (pinbank_to_outset_reg, $port:expr, PA) => ($port.outset0);
    (pinbank_to_outset_reg, $port:expr, PB) => ($port.outset1);
    (pinbank_to_outclr_reg, $port:expr, PA) => ($port.outclr0);
    (pinbank_to_outclr_reg, $port:expr, PB) => ($port.outclr1);
    (pinbank_to_dirset_reg, $port:expr, PA) => ($port.dirset0);
    (pinbank_to_dirset_reg, $port:expr, PB) => ($port.dirset1);
    (pinbank_to_dirclr_reg, $port:expr, PA) => ($port.dirclr0);
    (pinbank_to_dirclr_reg, $port:expr, PB) => ($port.dirclr1);

    (@bitmasking, $base:expr) => ($base);
    (@bitmasking, $base:expr, $next:expr $(, $other:expr)*) => (board_pin!(@bitmasking, ($base | (1 << $next)) $(, $other)*));
}
