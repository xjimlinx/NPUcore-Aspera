use super::config::CLOCK_FREQ;
use crate::hal::arch::set_timer;
use riscv::register::time;
pub const TICKS_PER_SEC: usize = 25;

/// Return current time measured by ticks, which is NOT divided by frequency.
pub fn get_time() -> usize {
    time::read()
}

/// Set next trigger.
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn get_clock_freq() -> usize {
    CLOCK_FREQ
}
