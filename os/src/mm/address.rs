use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use core::fmt::{self, Debug, Formatter};

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
/// 物理地址
pub struct PhysAddr(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
/// 虚拟地址
pub struct VirtAddr(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
/// 物理页号
pub struct PhysPageNum(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
/// 虚拟页号
pub struct VirtPageNum(pub usize);

/// Debug formatter for VirtAddr
impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VA")
            .field(&format_args!("{:#X}", self.0))
            .finish()
    }
}

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VPN")
            .field(&format_args!("{:#X}", self.0))
            .finish()
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PA")
            .field(&format_args!("{:#X}", self.0))
            .finish()
    }
}

impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PPN")
            .field(&format_args!("{:#X}", self.0))
            .finish()
    }
}

// 如下内容实现了上述类型与usize的双向转换
impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        v.0
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}

impl VirtAddr {
    /// 计算地址所在的页号（向下取整）
    pub fn floor(&self) -> VirtPageNum {
        let a = self.0 / PAGE_SIZE;
        VirtPageNum(a)
    }
    /// 计算地址所在的页号（向上取整）
    pub fn ceil(&self) -> VirtPageNum {
        let b = (self.0 - 1 + PAGE_SIZE) / PAGE_SIZE;
        VirtPageNum(b)
    }
    /// 计算地址在页内的偏移量
    pub fn page_offset(&self) -> usize {
        {
            let c = PAGE_SIZE - 1;
            self.0 & (c)
        }
    }
    /// 检查地址是否页对齐
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

/// 虚拟地址 转 虚拟页号
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}

/// 虚拟页号 转 虚拟地址
impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        let d = v.0 << PAGE_SIZE_BITS;
        Self(d)
    }
}

impl PhysAddr {
    /// 计算地址所在的页号（向下取整）
    pub fn floor(&self) -> PhysPageNum {
        let e = self.0 / PAGE_SIZE;
        PhysPageNum(e)
    }
    /// 计算地址所在的页号（向上取整）
    pub fn ceil(&self) -> PhysPageNum {
        let f = (self.0 - 1 + PAGE_SIZE) / PAGE_SIZE;
        PhysPageNum(f)
    }
    /// 计算地址在页内的偏移量
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// 检查地址是否页对齐
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

/// 物理地址 转 物理页号
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}

/// 物理页号 转 物理地址
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        let g = v.0 << PAGE_SIZE_BITS;
        Self(g)
    }
}

impl VirtPageNum {
    /// 获取页的起始地址
    pub fn start_addr(&self) -> VirtAddr {
        let f = self.0 << PAGE_SIZE_BITS;
        VirtAddr::from(f)
    }
    /// 通过页内偏移量计算地址
    pub fn offset(&self, offset: usize) -> VirtAddr {
        VirtAddr::from((self.0 << PAGE_SIZE_BITS) + offset)
    }
    /// 用于多级页表索引计算
    pub fn indexes<const T: usize>(&self) -> [usize; T] {
        // 获取虚拟页号
        let mut vpn = self.0;
        // 初始化索引数组
        let mut idx = [0usize; T];
        for i in (0..T).rev() {
            // 获取当前级别页表的索引
            idx[i] = vpn & 511;
            // 处理后右移9位
            vpn >>= 9;
        }
        idx
    }
}

/// 如下方法提供了从物理地址访问内存的能力
impl PhysAddr {
    /// 获取引用
    pub fn get_ref<T>(&self) -> &'static T {
        unsafe { (self.0 as *const T).as_ref().unwrap() }
    }
    /// 获取可变引用
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
    /// 以字节数组形式获取引用
    pub fn get_bytes_ref<T>(&self) -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(self.0 as *const u8, core::mem::size_of::<T>()) }
    }
    /// 以字节数组形式获取可变引用
    pub fn get_bytes_mut<T>(&self) -> &'static [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.0 as *mut u8, core::mem::size_of::<T>()) }
    }
}

impl PhysPageNum {
    /// 获取页的起始地址
    pub fn start_addr(&self) -> PhysAddr {
        PhysAddr::from(self.0 << PAGE_SIZE_BITS)
    }
    /// 通过页内偏移量计算地址
    pub fn offset(&self, offset: usize) -> PhysAddr {
        PhysAddr::from((self.0 << PAGE_SIZE_BITS) + offset)
    }
    /// 获取页表项数组
    pub fn get_pte_array<T>(&self) -> &'static mut [T] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut((pa.0) as *mut T, 512) }
    }
    /// 获取整个页的字节数组
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    /// 获取双字数组
    pub fn get_dwords_array(&self) -> &'static mut [u64] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u64, 512) }
    }
    /// 获取指定类型的可变引用
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = self.clone().into();
        pa.get_mut()
    }
}

/// 范围迭代器
/// 提供单步递增能力
pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
impl StepByOne for PhysPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone, Debug)]
/// 表示一个范围区间
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug + From<usize>,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}
pub type VPNRange = SimpleRange<VirtPageNum>;
pub type PPNRange = SimpleRange<PhysPageNum>;
