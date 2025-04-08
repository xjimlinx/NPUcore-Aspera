pub mod config;
pub mod kern_stack;
pub mod rv_pagetable;
pub mod sbi;
pub mod switch;
pub mod trap;

pub fn machine_init() {
    trap::init();
    trap::enable_timer_interrupt();
    set_next_trigger();
}

pub use tlb::tlb_invalidate;
use trap::trap_from_kernel;
mod tlb;
use core::arch::{asm, global_asm};

use crate::config::TRAMPOLINE;
use crate::mm::{frame_reserve, MemoryError, VirtAddr};
use crate::syscall::syscall;
use crate::task::{
    current_task, current_trap_cx, do_signal, do_wake_expired, suspend_current_and_run_next,
    Signals,
};
use crate::timer::set_next_trigger;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sepc, sie, stval, stvec,
};
pub use trap::context::MachineContext;

extern "C" {
    pub fn __alltraps();
    pub fn __restore();
    pub fn __call_sigreturn();
}

pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
