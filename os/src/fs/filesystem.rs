use crate::fs::ext4::BLOCK_SIZE;
use alloc::sync::Arc;
use core::ops::AddAssign;
use lazy_static::*;
use spin::Mutex;

use crate::drivers::BLOCK_DEVICE;

#[allow(unused, non_camel_case_types)]
#[derive(Debug)]
pub enum FS_Type {
    Null,
    Fat32,
    Ext4,
}

#[derive(Debug)]
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
    let mut buf = [0u8; BLOCK_SIZE];
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
