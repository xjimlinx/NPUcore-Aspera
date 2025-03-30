#[cfg(feature = "loongarch64")]
mod loongarch64;
#[cfg(feature = "loongarch64")]
pub use loongarch64::{
    board,
    board::MMIO,
    bootstrap_init,
    config,
    config::BUFFER_CACHE_NUM,
    config::KERNEL_HEAP_SIZE,
    config::MEMORY_END,
    console_flush,
    console_getchar,
    console_putchar,
    machine_init,
    shutdown,
    time::{get_clock_freq, get_time, TICKS_PER_SEC},
    KernelPageTableImpl,
    PageTableImpl,
    __switch,
    kstack_alloc,
    // tlb_global_invalidate,
    tlb_invalidate,
    trap::{
        get_bad_addr, get_bad_instruction, get_exception_cause, trap_handler, trap_return,
        MachineContext, TrapContext, TrapImpl, UserContext,
    },
    trap_cx_bottom_from_tid,
    ustack_bottom_from_tid,
    KernelStack,
    BLOCK_SZ,
};
#[cfg(feature = "riscv")]
mod riscv;
#[cfg(feature = "riscv")]
pub use riscv::{
    bootstrap_init, config,
    config::BUFFER_CACHE_NUM,
    config::MEMORY_END,
    console_flush, console_getchar, console_putchar, kstack_alloc, machine_init, shutdown,
    time::{get_clock_freq, get_time, TICKS_PER_SEC},
    tlb_invalidate,
    trap::{
        get_bad_addr, get_bad_instruction, get_exception_cause, trap_handler, trap_return,
        MachineContext, Trap, TrapContext, UserContext,
    },
    trap_cx_bottom_from_tid, ustack_bottom_from_tid, KernelStack, BLOCK_SZ,
};
