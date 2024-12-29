use core::mem::size_of;

const NAME_LIMIT: usize = 128;
#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// Native Linux directory entry structure.
/// In theory, the d_name may NOT have a fixed size and `d_name` may be arbitrarily lone.
pub struct Dirent {
    /// Inode 节点号
    pub d_ino: usize,
    /// Offset to next `linux_dirent`
    pub d_off: isize,
    /// Length of this `linux_dirent`
    pub d_reclen: u16,
    /// Type of the file
    pub d_type: u8,
    /// The Filename (null-terminated)
    /// # Note
    /// We use fix-sized d_name array.
    pub d_name: [u8; NAME_LIMIT],
}

impl Dirent {
    /// Offset to next `linux_dirent`
    pub fn new(d_ino: usize, d_off: isize, d_type: u8, d_name: &str) -> Self {
        let mut dirent = Self {
            d_ino,
            d_off,
            d_reclen: size_of::<Self>() as u16,
            d_type,
            d_name: [0; NAME_LIMIT],
        };
        dirent.d_name[0..d_name.len()].copy_from_slice(d_name.as_bytes());
        dirent
    }
}
