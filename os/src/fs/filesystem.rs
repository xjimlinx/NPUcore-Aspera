use core::ops::AddAssign;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

#[allow(unused, non_camel_case_types)]
pub enum FS_Type {
    Null,
    Fat32,
    Ext4,
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
