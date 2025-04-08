use core::arch::asm;

#[inline(always)]
pub fn tlb_invalidate() {
    unsafe {
        asm!("sfence.vma");
    }
}