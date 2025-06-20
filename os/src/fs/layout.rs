use crate::{hal::BLOCK_SZ, timer::TimeSpec};

bitflags! {
    pub struct OpenFlags: u32 {
        const O_RDONLY      =   0o0;
        const O_WRONLY      =   0o1;
        const O_RDWR        =   0o2;

        const O_CREAT       =   0o100;
        const O_EXCL        =   0o200;
        const O_NOCTTY      =   0o400;
        const O_TRUNC       =   0o1000;

        const O_APPEND      =   0o2000;
        const O_NONBLOCK    =   0o4000;
        const O_DSYNC       =   0o10000;
        const O_SYNC        =   0o4010000;
        const O_RSYNC       =   0o4010000;
        const O_DIRECTORY   =   0o200000;
        const O_NOFOLLOW    =   0o400000;
        const O_CLOEXEC     =   0o2000000;
        const O_ASYNC       =   0o20000;
        const O_DIRECT      =   0o40000;
        const O_LARGEFILE   =   0o100000;
        const O_NOATIME     =   0o1000000;
        const O_PATH        =   0o10000000;
        const O_TMPFILE     =   0o20200000;
    }
}

bitflags! {
    pub struct SeekWhence: u32 {
        const SEEK_SET  =   0; /* set to offset bytes.  */
        const SEEK_CUR  =   1; /* set to its current location plus offset bytes.  */
        const SEEK_END  =   2; /* set to the size of the file plus offset bytes.  */
    }
}

bitflags! {
    pub struct StatMode: u32 {
        ///bit mask for the file type bit field
        const S_IFMT    =   0o170000;
        ///socket
        const S_IFSOCK  =   0o140000;
        ///symbolic link
        const S_IFLNK   =   0o120000;
        ///regular file
        const S_IFREG   =   0o100000;
        ///block device
        const S_IFBLK   =   0o060000;
        ///directory
        const S_IFDIR   =   0o040000;
        ///character device
        const S_IFCHR   =   0o020000;
        ///FIFO
        const S_IFIFO   =   0o010000;

        ///set-user-ID bit (see execve(2))
        const S_ISUID   =   0o4000;
        ///set-group-ID bit (see below)
        const S_ISGID   =   0o2000;
        ///sticky bit (see below)
        const S_ISVTX   =   0o1000;

        ///owner has read, write, and execute permission
        const S_IRWXU   =   0o0700;
        ///owner has read permission
        const S_IRUSR   =   0o0400;
        ///owner has write permission
        const S_IWUSR   =   0o0200;
        ///owner has execute permission
        const S_IXUSR   =   0o0100;

        ///group has read, write, and execute permission
        const S_IRWXG   =   0o0070;
        ///group has read permission
        const S_IRGRP   =   0o0040;
        ///group has write permission
        const S_IWGRP   =   0o0020;
        ///group has execute permission
        const S_IXGRP   =   0o0010;

        ///others (not in group) have read, write,and execute permission
        const S_IRWXO   =   0o0007;
        ///others have read permission
        const S_IROTH   =   0o0004;
        ///others have write permission
        const S_IWOTH   =   0o0002;
        ///others have execute permission
        const S_IXOTH   =   0o0001;
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// Store the file attributes from a supported file.
pub struct Stat {
    /// ID of device containing file
    st_dev: u64,
    /// Inode number
    st_ino: u64,
    /// File type and mode
    st_mode: u32,
    /// Number of hard links
    st_nlink: u32,
    /// User ID of the file's owner.
    st_uid: u32,
    /// Group ID of the file's group.
    st_gid: u32,
    /// Device ID (if special file)
    st_rdev: u64,
    __pad: u64,
    /// Size of file, in bytes.
    st_size: i64,
    /// Optimal block size for I/O.
    st_blksize: u32,
    __pad2: i32,
    /// Number 512-byte blocks allocated.
    st_blocks: u64,
    /// Backward compatibility. Used for time of last access.
    st_atime: TimeSpec,
    /// Time of last modification.
    st_mtime: TimeSpec,
    /// Time of last status change.
    st_ctime: TimeSpec,
    __unused: u64,
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// Store the file attributes from a supported file.
pub struct Statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    __statx_pad1: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    __statx_pad2: u64,
    __statx_pad3: [u64; 12],
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __statx_timestamp_pad1: [i32; 1],
}

impl Statx {
    #![allow(unused)]
    /// Get the inode number described in the `Stat`
    pub fn get_ino(&self) -> usize {
        self.stx_ino as usize
    }
    pub fn get_size(&self) -> usize {
        self.stx_size as usize
    }
    pub fn new(
        stx_mask: u32,
        stx_nlink: u32,
        stx_mode: u16,
        stx_ino: u64,
        stx_size: u64,
        stx_atime_sec: i64,
        stx_ctime_sec: i64,
        stx_mtime_sec: i64,
        stx_rdev_major: u32,
        stx_rdev_minor: u32,
        stx_dev_major: u32,
        stx_dev_minor: u32,
    ) -> Self {
        const BLK_SIZE: u32 = BLOCK_SZ as u32;
        Self {
            stx_mask: stx_mask,
            stx_blksize: BLK_SIZE as u32,
            stx_attributes: 0,
            stx_nlink,
            stx_uid: 0,
            stx_gid: 0,
            stx_mode,
            __statx_pad1: [0 as u16; 1],
            stx_ino,
            stx_size,
            stx_blocks: (stx_size as u64 + BLK_SIZE as u64 - 1) / BLK_SIZE as u64,
            stx_attributes_mask: 0,
            stx_atime: StatxTimestamp {
                tv_sec: stx_atime_sec,
                tv_nsec: 0,
                __statx_timestamp_pad1: [0; 1],
            },
            stx_btime: StatxTimestamp {
                tv_sec: stx_ctime_sec,
                tv_nsec: 0,
                __statx_timestamp_pad1: [0; 1],
            },
            stx_ctime: StatxTimestamp {
                tv_sec: stx_ctime_sec,
                tv_nsec: 0,
                __statx_timestamp_pad1: [0; 1],
            },
            stx_mtime: StatxTimestamp {
                tv_sec: stx_mtime_sec,
                tv_nsec: 0,
                __statx_timestamp_pad1: [0; 1],
            },
            stx_rdev_major,
            stx_rdev_minor,
            stx_dev_major,
            stx_dev_minor,
            stx_mnt_id: 0,
            __statx_pad2: 0,
            __statx_pad3: [0 as u64; 12],
        }
    }
}
#[allow(unused)]
impl Stat {
    /// Get the inode number described in the `Stat`
    pub fn get_ino(&self) -> usize {
        self.st_ino as usize
    }
    pub fn get_size(&self) -> usize {
        self.st_size as usize
    }
    pub fn get_mode(&self) -> u32 {
        self.st_mode
    }
    pub fn get_nlink(&self) -> u32 {
        self.st_nlink
    }
    pub fn get_dev(&self) -> u32 {
        self.st_dev as u32
    }
    pub fn get_rdev(&self) -> u32 {
        self.st_rdev as u32
    }
    pub fn get_atime(&self) -> usize {
        self.st_atime.tv_sec as usize
    }
    pub fn get_mtime(&self) -> usize {
        self.st_mtime.tv_sec as usize
    }
    pub fn get_ctime(&self) -> usize {
        self.st_ctime.tv_sec as usize
    }

    pub fn new(
        st_dev: u64,
        st_ino: u64,
        st_mode: u32,
        st_nlink: u32,
        st_rdev: u64,
        st_size: i64,
        st_atime_sec: i64,
        st_mtime_sec: i64,
        st_ctime_sec: i64,
    ) -> Self {
        const BLK_SIZE: u32 = BLOCK_SZ as u32;
        Self {
            st_dev,
            st_ino,
            st_mode,
            st_nlink,
            st_uid: 0,
            st_gid: 0,
            st_rdev,
            __pad: 0,
            st_size,
            st_blksize: BLK_SIZE as u32,
            __pad2: 0,
            st_blocks: (st_size as u64 + BLK_SIZE as u64 - 1) / BLK_SIZE as u64,
            st_atime: TimeSpec {
                tv_sec: st_atime_sec as usize,
                tv_nsec: 0,
            },
            st_mtime: TimeSpec {
                tv_sec: st_mtime_sec as usize,
                tv_nsec: 0,
            },
            st_ctime: TimeSpec {
                tv_sec: st_ctime_sec as usize,
                tv_nsec: 0,
            },
            __unused: 0,
        }
    }
}
