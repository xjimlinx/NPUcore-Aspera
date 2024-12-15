use crate::fs::ext4::ext4fs::Ext4FileSystem;
use crate::fs::fat32::EasyFileSystem;
use crate::fs::vfs::VFS;
use alloc::sync::Arc;
use core::ops::AddAssign;
use lazy_static::*;
use spin::Mutex;

use crate::drivers::BLOCK_DEVICE;

use super::ext4::BlockCacheManager;
use super::BlockDevice;

#[allow(unused, non_camel_case_types)]
pub enum FS_Type {
    Null,
    Fat32,
    Ext4,
}

impl FS_Type {
    pub fn mount_fs(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<dyn VFS> {
        let fs_type = pre_mount();
        match fs_type {
            FS_Type::Fat32 => EasyFileSystem::open(block_device, index_cache_mgr),
            FS_Type::Ext4 => Ext4FileSystem::open(block_device, index_cache_mgr),
            FS_Type::Null => panic!("no filesystem found"),
        }
    }
}

pub struct FileSystem {
    pub fs_id: usize,
    pub fs_type: FS_Type,
}

lazy_static! {
    static ref FS_ID_COUNTER: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

impl FileSystem {
    pub fn new(fs_type: FS_Type) -> Self {
        FS_ID_COUNTER.lock().add_assign(1);
        let fs_id = *FS_ID_COUNTER.lock();
        Self { fs_id, fs_type }
    }
}

pub fn pre_mount() -> FS_Type {
    // 先读取块设备的第512个字节看是不是0x55AA
    // 来判断是不是fat32
    // 如果是fat32，返回FS_Type::Fat32
    // 否则尝试获取超级块的魔数，如果是0xEF53，返回FS_Type::Ext4
    // 否则返回FS_Type::Null
    let block_device = BLOCK_DEVICE.clone();
    let mut buf = [0u8; 2048];
    block_device.read_block(0, &mut buf);
    // 判断第512个字节是不是0x55AA
    if buf[510] == 0x55 && buf[511] == 0xAA {
        println!("[fs] found fat32 filesystem");
        return FS_Type::Fat32;
    } else {
        let superblock_offset = 1024;
        let magic_number_high_index = superblock_offset + 56;
        let magic_number_low_index = superblock_offset + 57;
        let magic_number =
            u16::from_le_bytes([buf[magic_number_high_index], buf[magic_number_low_index]]);
        println!("[fs] read magic number: {}", magic_number);
        if magic_number == 0xEF53 {
            println!("[fs] found ext4 filesystem");
            return FS_Type::Ext4;
        }
    }
    println!("[fs] no filesystem found");
    FS_Type::Null
}
