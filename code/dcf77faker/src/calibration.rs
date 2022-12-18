//! Functionality to obtain calibration values from NVM.


fn read_calibration_area() -> u32 {
    // SAM L21 datasheet, § 11.4
    let calibration_area_ptr = 0x0080_6020 as *const u32;
    unsafe { *calibration_area_ptr }
}

/// The ADC linearity calibration value.
///
/// Bits 2:0; to be stored into `ADC.calib.biasrefbuf`.
pub(crate) fn adc_linearity() -> u8 {
    ((read_calibration_area() >> 0) & 0b111) as u8
}

/// The ADC bias calibration value.
///
/// Bits 5:3; to be stored into `ADC.calib.biascomp`.
pub(crate) fn adc_bias() -> u8 {
    ((read_calibration_area() >> 3) & 0b111) as u8
}

/// The 32kHz internal oscillator calibration value.
///
/// Bits 12:6; to be stored into `OSC32KCTRL.osc32k.calib`.
pub(crate) fn osc32k() -> u8 {
    ((read_calibration_area() >> 6) & 0b111_1111) as u8
}

/// The USB TRANSN calibration value.
///
/// Bits 17:13; to be stored into `USB.$mode().padcal.transn`.
pub(crate) fn usb_transn() -> u8 {
    ((read_calibration_area() >> 13) & 0b1_1111) as u8
}

/// The USB TRANSP calibration value.
///
/// Bits 22:18; to be stored into `USB.$mode().padcal.transp`.
pub(crate) fn usb_transp() -> u8 {
    ((read_calibration_area() >> 18) & 0b1_1111) as u8
}

/// The USB TRIM calibration value.
///
/// Bits 25:23; to be stored into `USB.$mode().padcal.trim`.
pub(crate) fn usb_trim() -> u8 {
    ((read_calibration_area() >> 23) & 0b111) as u8
}

/// The DFLL48M coarse calibration value.
///
/// Bits 31:26; to be stored into `OSCCTRL.dfllval.coarse`.
pub(crate) fn dfll48m_coarse() -> u8 {
    ((read_calibration_area() >> 26) & 0b11_1111) as u8
}
