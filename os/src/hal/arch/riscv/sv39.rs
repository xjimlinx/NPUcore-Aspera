use crate::mm::{address::*, frame_alloc, FrameTracker, MapPermission, PageTable};
use alloc::{sync::Arc, vec::Vec};
use bitflags::*;
use core::arch::asm;
use riscv::register::satp;

#[inline(always)]
pub fn tlb_invalidate() {
    unsafe {
        asm!("sfence.vma");
    }
}
bitflags! {
    /// Page Table Entry flags
    pub struct PTEFlags: u8 {
    /// Valid Bit
        const V = 1 << 0;
    /// Readable Bit
        const R = 1 << 1;
    /// Writable Bit
        const W = 1 << 2;
    /// Executable Bit
        const X = 1 << 3;
    /// User Space Bit, true if it can be accessed from user space.
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
    /// Dirty Bit, true if it is modified.
        const D = 1 << 7;
    }
}

/// Page Table Entry
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Sv39PageTableEntry {
    pub bits: usize,
}

impl Sv39PageTableEntry {
    const PPN_MASK: usize = ((1usize << 44) - 1) << 10;
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Sv39PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        Sv39PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        ((self.bits & Self::PPN_MASK) >> 10).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn is_dirty(&self) -> bool {
        (self.flags() & PTEFlags::D) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
    pub fn clear_access(&mut self) {
        self.bits &= !(PTEFlags::A.bits() as usize);
    }
    pub fn clear_dirty(&mut self) {
        self.bits &= !(PTEFlags::D.bits() as usize);
    }
    pub fn revoke_read(&mut self) {
        self.bits &= !(PTEFlags::R.bits() as usize);
    }
    pub fn revoke_write(&mut self) {
        self.bits &= !(PTEFlags::W.bits() as usize);
    }
    pub fn revoke_execute(&mut self) {
        self.bits &= !(PTEFlags::X.bits() as usize);
    }
    pub fn set_permission(&mut self, flags: MapPermission) {
        self.bits = (self.bits & 0xffff_ffff_ffff_ffe1) | (flags.bits() as usize)
        // | ((PTEFlags::A.bits() | PTEFlags::D.bits()) as usize)
    }
    pub fn set_ppn(&mut self, ppn: PhysPageNum) {
        self.bits = (self.bits & !Self::PPN_MASK) | ((ppn.0 << 10) & Self::PPN_MASK)
    }
}

pub struct Sv39PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<Arc<FrameTracker>>,
}

/// Assume that it won't encounter oom when creating/mapping.
impl Sv39PageTable {
    /// Find the page in the page table, creating the page on the way if not exists.
    /// Note: It does NOT create the terminal node. The caller must verify its validity and create according to his own needs.
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut Sv39PageTableEntry> {
        let idxs: [usize; 3] = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut Sv39PageTableEntry> = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                // this condition is used to make sure the
                //returning predication is put before validity to quit before creating the terminal page entry.
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                // xein TODO:
                // 这里有问题
                // *pte = Sv39PageTableEntry::new(frame.ppn, PTEFlags::V | PTEFlags::A | PTEFlags::D);
                *pte = Sv39PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find the page table entry denoted by vpn, returning Some(&_) if found or None if not.
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&Sv39PageTableEntry> {
        let idxs: [usize; 3] = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&Sv39PageTableEntry> = None;
        for i in 0..3 {
            let pte = &ppn.get_pte_array::<Sv39PageTableEntry>()[idxs[i]];
            if !pte.is_valid() {
                return None;
            }
            if i == 2 {
                result = Some(pte);
                break;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find and return reference the page table entry denoted by `vpn`, `None` if not found.
    fn find_pte_refmut(&self, vpn: VirtPageNum) -> Option<&mut Sv39PageTableEntry> {
        let idxs: [usize; 3] = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut Sv39PageTableEntry> = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array::<Sv39PageTableEntry>()[idxs[i]];
            if !pte.is_valid() {
                return None;
            }
            if i == 2 {
                result = Some(pte);
                break;
            }
            ppn = pte.ppn();
        }
        result
    }
}
/// Assume that it won't encounter oom when creating/mapping.
impl PageTable for Sv39PageTable {
    fn new_kern_space() -> Self
    where
        Self: Sized,
    {
        let frame = frame_alloc().unwrap();
        Sv39PageTable {
            root_ppn: frame.ppn,
            frames: {
                let mut vec = Vec::with_capacity(256);
                vec.push(frame);
                vec
            },
        }
    }
    fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Sv39PageTable {
            root_ppn: frame.ppn,
            frames: {
                let mut vec = Vec::with_capacity(256);
                vec.push(frame);
                vec
            },
        }
    }
    /// Create an empty page table from `satp`
    /// # Argument
    /// * `satp` Supervisor Address Translation & Protection reg. that points to the physical page containing the root page.
    fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Predicate for the valid bit.
    fn is_mapped(&mut self, vpn: VirtPageNum) -> bool {
        if let Some(i) = self.find_pte(vpn) {
            if i.is_valid() {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    /// Find the page in the page table, creating the page on the way if not exists.
    /// Note: It does NOT create the terminal node. The caller must verify its validity and create according to his own needs.
    #[allow(unused)]
    /// Map the `vpn` to `ppn` with the `flags`.
    /// # Note
    /// Allocation should be done elsewhere.
    /// # Exceptions
    /// Panics if the `vpn` is mapped.
    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: MapPermission) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = Sv39PageTableEntry::new(
            ppn,
            // xein TODO:
            PTEFlags::from_bits(flags.bits()).unwrap() | PTEFlags::V | PTEFlags::A | PTEFlags::D,
            // PTEFlags::from_bits(flags.bits()).unwrap() | PTEFlags::V,
        );
    }
    #[allow(unused)]
    /// Unmap the `vpn` to `ppn` with the `flags`.
    /// # Exceptions
    /// Panics if the `vpn` is NOT mapped (invalid).
    fn unmap(&mut self, vpn: VirtPageNum) {
        //tlb_invalidate();
        let pte = self.find_pte_refmut(vpn).unwrap(); // was `self.find_creat_pte(vpn).unwrap()`;
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = Sv39PageTableEntry::empty();
    }
    /// Translate the `vpn` into its corresponding `Some(PageTableEntry)` if exists
    /// `None` is returned if nothing is found.
    fn translate(&self, vpn: VirtPageNum) -> Option<PhysPageNum> {
        // This is not the same map as we defined just now...
        // It is the map for func. programming.
        self.find_pte(vpn).map(|pte| pte.ppn())
    }
    /// Translate the virtual address into its corresponding `PhysAddr` if mapped in current page table.
    /// `None` is returned if nothing is found.
    fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    fn block_and_ret_mut(&self, vpn: VirtPageNum) -> Option<PhysPageNum> {
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.revoke_write();
            Some(pte.ppn())
        } else {
            None
        }
    }
    /// Return the physical token to current page.
    fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
    fn revoke_read(&mut self, vpn: VirtPageNum) -> Result<(), ()> {
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.revoke_read();
            Ok(())
        } else {
            Err(())
        }
    }
    fn revoke_write(&mut self, vpn: VirtPageNum) -> Result<(), ()> {
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.revoke_write();
            Ok(())
        } else {
            Err(())
        }
    }
    fn revoke_execute(&mut self, vpn: VirtPageNum) -> Result<(), ()> {
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.revoke_execute();
            Ok(())
        } else {
            Err(())
        }
    }
    fn set_ppn(&mut self, vpn: VirtPageNum, ppn: PhysPageNum) -> Result<(), ()> {
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.set_ppn(ppn);
            Ok(())
        } else {
            Err(())
        }
    }
    fn set_pte_flags(&mut self, vpn: VirtPageNum, flags: MapPermission) -> Result<(), ()> {
        //tlb_invalidate();
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.set_permission(flags);
            Ok(())
        } else {
            Err(())
        }
    }
    fn clear_access_bit(&mut self, vpn: VirtPageNum) -> Result<(), ()> {
        tlb_invalidate();
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.clear_access();
            Ok(())
        } else {
            Err(())
        }
    }
    fn clear_dirty_bit(&mut self, vpn: VirtPageNum) -> Result<(), ()> {
        tlb_invalidate();
        if let Some(pte) = self.find_pte_refmut(vpn) {
            pte.clear_dirty();
            Ok(())
        } else {
            Err(())
        }
    }
    fn activate(&self) {
        // TODO:
        let satp = self.token();
        // Problem in here.
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        };
    }
    fn is_valid(&self, vpn: VirtPageNum) -> Option<bool> {
        self.find_pte(vpn).map(|pte| pte.is_valid())
    }
    fn is_dirty(&self, vpn: VirtPageNum) -> Option<bool> {
        self.find_pte(vpn).map(|pte| pte.is_dirty())
    }
    fn readable(&self, vpn: VirtPageNum) -> Option<bool> {
        self.find_pte(vpn).map(|pte| pte.readable())
    }
    fn writable(&self, vpn: VirtPageNum) -> Option<bool> {
        self.find_pte(vpn).map(|pte| pte.writable())
    }
    fn executable(&self, vpn: VirtPageNum) -> Option<bool> {
        self.find_pte(vpn).map(|pte| pte.executable())
    }
}
