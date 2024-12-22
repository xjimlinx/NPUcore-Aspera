use alloc::vec;

use super::ext4fs::Ext4FileSystem;

impl Ext4FileSystem {
    /// 尝试打开一个文件并读取内容
    /// 读取 2048 个字节
    pub fn test_get_file(&self, path: &str) {
        // 读取大小 1G
        let read_size = 2048;
        let mut read_buf = vec![0u8; read_size as usize];
        let child_inode = self.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
        let mut data = vec![0u8; read_size as usize];
        // 读取文件内容
        let read_data = self.read_at(child_inode, 0 as usize, &mut data);
        println!("read data  {:?}", &data[..10]);
    }
}
