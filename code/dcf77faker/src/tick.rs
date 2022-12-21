use core::time::Duration;

use cortex_m::Peripherals;
use cortex_m_rt::exception;

use crate::init::CORE_CLOCK_SPEED_HZ;
use crate::sync_vcell::SyncVolatileCell;


pub(crate) static TICK_CLOCK: SyncVolatileCell<u32> = SyncVolatileCell::new(0);


#[exception]
unsafe fn SysTick() {
    TICK_CLOCK.set(TICK_CLOCK.get().wrapping_add(1))
}

pub fn enable_tick_clock(core_peripherals: &mut Peripherals) {
    const SYST_CSR_ENABLE_ENABLED: u32 = 1 << 0;
    const SYST_CSR_TICKINT_ENABLED: u32 = 1 << 1;
    const SYST_CSR_CLKSOURCE_MCK: u32 = 1 << 2;

    unsafe {
        core_peripherals.SYST.rvr.write(CORE_CLOCK_SPEED_HZ / 1000)
    };
    unsafe {
        core_peripherals.SYST.csr.write(
            SYST_CSR_ENABLE_ENABLED
            | SYST_CSR_TICKINT_ENABLED
            | SYST_CSR_CLKSOURCE_MCK
        )
    };
}

#[inline]
pub fn delay(duration: Duration) {
    let ms_u128 = duration.as_millis();
    let ms = if ms_u128 > u32::MAX.into() {
        u32::MAX
    } else {
        ms_u128 as u32
    };

    let start = TICK_CLOCK.get();
    while TICK_CLOCK.get() < start + ms {
        // nop
    }
}
