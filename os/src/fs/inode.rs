pub struct InodeTime {
    create_time: u64,
    access_time: u64,
    modify_time: u64,
}
#[allow(unused)]
impl InodeTime {
    pub fn new() -> Self {
        Self {
            create_time: 0,
            access_time: 0,
            modify_time: 0,
        }
    }
    /// Set the inode time's create time.
    pub fn set_create_time(&mut self, create_time: u64) {
        self.create_time = create_time;
    }

    /// Get a reference to the inode time's create time.
    pub fn create_time(&self) -> &u64 {
        &self.create_time
    }

    /// Set the inode time's access time.
    pub fn set_access_time(&mut self, access_time: u64) {
        self.access_time = access_time;
    }

    /// Get a reference to the inode time's access time.
    pub fn access_time(&self) -> &u64 {
        &self.access_time
    }

    /// Set the inode time's modify time.
    pub fn set_modify_time(&mut self, modify_time: u64) {
        self.modify_time = modify_time;
    }

    /// Get a reference to the inode time's modify time.
    pub fn modify_time(&self) -> &u64 {
        &self.modify_time
    }
}