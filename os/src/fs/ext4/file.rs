use super::*;
use alloc::vec::Vec;
use block_group::Block;
use ext4fs::Ext4FileSystem;
use path::path_check;
use spin::RwLock;

pub struct Ext4FileContent {
    /// The size of the file.
    pub size: u32,
    /// The block list.
    block_list: Vec<u32>,
    /// The inode number.
    inode: u32,
}

impl Ext4FileContent {
    pub fn new(size: u32, block_list: Vec<u32>, inode: u32) -> Self {
        Self {
            size,
            block_list,
            inode,
        }
    }

    pub fn get_block_list(&self) -> &Vec<u32> {
        &self.block_list
    }
}

use core::cmp::min;

#[allow(unused)]
pub struct FileAttr {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: u32,
    /// Time of last modification
    pub mtime: u32,
    /// Time of last change
    pub ctime: u32,
    /// Time of creation (macOS only)
    pub crtime: u32,
    /// Time of last status change
    pub chgtime: u32,
    /// Backup time (macOS only)
    pub bkuptime: u32,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: InodeFileType,
    /// Permissions
    pub perm: InodePerm,
    /// Number of hard links
    pub nlink: u32,
    /// User id
    pub uid: u32,
    /// Group id
    pub gid: u32,
    /// Rdev
    pub rdev: u32,
    /// Block size
    pub blksize: u32,
    /// Flags (macOS only, see chflags(2))
    pub flags: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
        FileAttr {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            chgtime: 0,
            bkuptime: 0,
            kind: InodeFileType::S_IFREG,
            perm: InodePerm::S_IREAD | InodePerm::S_IWRITE | InodePerm::S_IEXEC,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
}

#[allow(unused)]
impl FileAttr {
    pub fn from_inode_ref(inode_ref: &Ext4InodeRef) -> FileAttr {
        let inode_num = inode_ref.inode_num;
        let inode = inode_ref.inode;
        FileAttr {
            ino: inode_num as u64,
            size: inode.size(),
            blocks: inode.blocks_count(),
            atime: inode.atime(),
            mtime: inode.mtime(),
            ctime: inode.ctime(),
            crtime: inode.i_crtime(),
            // todo: chgtime, bkuptime
            chgtime: 0,
            bkuptime: 0,
            kind: inode.file_type(),
            perm: inode.file_perm(), // Extract permission bits
            nlink: inode.links_count() as u32,
            uid: inode.uid() as u32,
            gid: inode.gid() as u32,
            rdev: inode.faddr(),
            blksize: BLOCK_SIZE as u32,
            flags: inode.flags(),
        }
    }
}

// #ifdef __i386__
// struct stat {
// 	unsigned long  st_dev;
// 	unsigned long  st_ino;
// 	unsigned short st_mode;
// 	unsigned short st_nlink;
// 	unsigned short st_uid;
// 	unsigned short st_gid;
// 	unsigned long  st_rdev;
// 	unsigned long  st_size;
// 	unsigned long  st_blksize;
// 	unsigned long  st_blocks;
// 	unsigned long  st_atime;
// 	unsigned long  st_atime_nsec;
// 	unsigned long  st_mtime;
// 	unsigned long  st_mtime_nsec;
// 	unsigned long  st_ctime;
// 	unsigned long  st_ctime_nsec;
// 	unsigned long  __unused4;
// 	unsigned long  __unused5;
// };

#[repr(C)]
pub struct LinuxStat {
    st_dev: u32,        // ID of device containing file
    st_ino: u32,        // Inode number
    st_mode: u16,       // File type and mode
    st_nlink: u16,      // Number of hard links
    st_uid: u16,        // User ID of owner
    st_gid: u16,        // Group ID of owner
    st_rdev: u32,       // Device ID (if special file)
    st_size: u32,       // Total size, in bytes
    st_blksize: u32,    // Block size for filesystem I/O
    st_blocks: u32,     // Number of 512B blocks allocated
    st_atime: u32,      // Time of last access
    st_atime_nsec: u32, // Nanoseconds part of last access time
    st_mtime: u32,      // Time of last modification
    st_mtime_nsec: u32, // Nanoseconds part of last modification time
    st_ctime: u32,      // Time of last status change
    st_ctime_nsec: u32, // Nanoseconds part of last status change time
    __unused4: u32,     // Unused field
    __unused5: u32,     // Unused field
}

impl LinuxStat {
    pub fn from_inode_ref(inode_ref: &Ext4InodeRef) -> LinuxStat {
        let inode_num = inode_ref.inode_num;
        let inode = &inode_ref.inode;

        LinuxStat {
            st_dev: 0,
            st_ino: inode_num,
            st_mode: inode.mode,
            st_nlink: inode.links_count(),
            st_uid: inode.uid(),
            st_gid: inode.gid(),
            st_rdev: 0,
            st_size: inode.size() as u32,
            st_blksize: 4096, // 假设块大小为4096字节
            st_blocks: inode.blocks_count() as u32,
            st_atime: inode.atime(),
            st_atime_nsec: 0,
            st_mtime: inode.mtime(),
            st_mtime_nsec: 0,
            st_ctime: inode.ctime(),
            st_ctime_nsec: 0,
            __unused4: 0,
            __unused5: 0,
        }
    }
}

impl Ext4FileSystem {
    /// Link a child inode to a parent directory
    ///
    /// Params:
    /// parent: &mut Ext4InodeRef - parent directory inode reference
    /// child: &mut Ext4InodeRef - child inode reference
    /// name: &str - name of the child inode
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn link(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
    ) -> Result<usize, isize> {
        // Add a directory entry in the parent directory pointing to the child inode

        // at this point should insert to existing block
        self.dir_add_entry(parent, child, name)?;
        self.write_back_inode_without_csum(parent);

        // If this is the first link. add '.' and '..' entries
        if child.inode.is_dir() {
            // let child_ref = child.clone();
            let new_child_ref = Ext4InodeRef {
                inode_num: child.inode_num,
                inode: child.inode,
            };

            // at this point child need a new block
            self.dir_add_entry(child, &new_child_ref, ".")?;

            // at this point should insert to existing block
            self.dir_add_entry(child, &new_child_ref, "..")?;

            child.inode.set_links_count(2);
            let link_cnt = parent.inode.links_count() + 1;
            parent.inode.set_links_count(link_cnt);

            return Ok(EOK);
        }

        // Increment the link count of the child inode
        let link_cnt = child.inode.links_count() + 1;
        child.inode.set_links_count(link_cnt);

        Ok(EOK)
    }

    /// 创建一个新inode并将其链接到其父目录
    /// # 参数
    /// + parent: u32 - 父目录的inode号
    /// + name: &str - 新文件的名称
    /// + mode: u16 - 文件模式
    ///
    /// # 返回值:
    /// + `Result<Ext4InodeRef>` - 新文件的inode
    pub fn create(&self, parent: u32, name: &str, inode_mode: u16) -> Result<Ext4InodeRef, isize> {
        // 获取父目录的inode
        let mut parent_inode_ref = self.get_inode_ref(parent);

        // 创建一个新inode
        let init_child_ref = self.create_inode(inode_mode)?;

        // 写回inode
        // TODO: 在使用LoongsonNand的时候读和写的数据不一样
        self.write_back_inode_without_csum(&init_child_ref);
        let mut child_inode_ref = self.get_inode_ref(init_child_ref.inode_num);

        // 链接新 inode 到父目录
        self.link(&mut parent_inode_ref, &mut child_inode_ref, name)?;

        // 写回父目录 inode
        self.write_back_inode(&mut parent_inode_ref);
        // 写回新 inode
        self.write_back_inode(&mut child_inode_ref);

        Ok(child_inode_ref)
    }

    /// 创建inode
    /// # 参数
    /// + inode_mode: inode类型
    /// # 返回值
    /// + 新inode
    pub fn create_inode(&self, inode_mode: u16) -> Result<Ext4InodeRef, isize> {
        // 匹配新inode的文件类型
        let inode_file_type_bits = inode_mode & EXT4_INODE_MODE_TYPE_MASK;
        // println!(
        //     "[kernel create_inode] inode_mode {:?}, {:?}",
        //     inode_mode,
        //     InodeFileType::from_bits(inode_file_type_bits)
        // );
        let inode_file_type = match InodeFileType::from_bits(inode_file_type_bits) {
            Some(file_type) => file_type,
            None => InodeFileType::S_IFREG,
        };
        // println!("[kernel create_inode] {:?}", inode_file_type);

        // 判断是否是文件夹
        let is_dir = inode_file_type == InodeFileType::S_IFDIR;

        // 分配inode
        let inode_num = self.alloc_inode(is_dir);
        if let Err(e) = inode_num {
            return Err(e);
        }

        // 初始化inode
        let mut inode = Ext4Inode::default();

        // 设置文件类型和权限
        inode.set_mode(inode_mode | 0o777);

        // set extra size
        let inode_size = self.superblock.inode_size();
        let extra_size = self.superblock.extra_size();
        if inode_size > EXT4_GOOD_OLD_INODE_SIZE {
            let extra_size = extra_size;
            inode.set_i_extra_isize(extra_size);
        }

        // set extent
        inode.set_flags(EXT4_INODE_FLAG_EXTENTS as u32);
        inode.extent_tree_init();

        let inode_ref = Ext4InodeRef {
            inode_num: inode_num.unwrap(),
            inode,
        };

        Ok(inode_ref)
    }

    /// create a new inode and link it to the parent directory
    ///
    /// Params:
    /// parent: u32 - inode number of the parent directory
    /// name: &str - name of the new file
    /// mode: u16 - file mode
    /// uid: u32 - user id
    /// gid: u32 - group id
    ///
    /// Returns:
    pub fn create_with_attr(
        &self,
        parent: u32,
        name: &str,
        inode_mode: u16,
        uid: u16,
        gid: u16,
    ) -> Result<Ext4InodeRef, isize> {
        let mut parent_inode_ref = self.get_inode_ref(parent);

        // let mut child_inode_ref = self.create_inode(inode_mode)?;
        let mut init_child_ref = self.create_inode(inode_mode)?;

        init_child_ref.inode.set_uid(uid);
        init_child_ref.inode.set_gid(gid);

        self.write_back_inode_without_csum(&init_child_ref);
        // load new
        let mut child_inode_ref = self.get_inode_ref(init_child_ref.inode_num);

        self.link(&mut parent_inode_ref, &mut child_inode_ref, name)?;

        self.write_back_inode(&mut parent_inode_ref);
        self.write_back_inode(&mut child_inode_ref);

        Ok(child_inode_ref)
    }

    /// 从指定文件的某个偏移位置开始读取数据
    /// # 参数
    /// + inode: u32 - 文件的inode号
    /// + offset: usize - offset from where to read
    /// + read_buf: &mut [u8] - 存储读取的数据的buffer
    /// # 返回值
    /// `Result<usize>`：读取的字节数
    pub fn read_at(&self, inode: u32, offset: usize, read_buf: &mut [u8]) -> Result<usize, isize> {
        // 缓冲区为空，返回 0
        let mut read_buf_len = read_buf.len();
        if read_buf_len == 0 {
            return Ok(0);
        }

        // 获取ext4inoderef对象
        let inode_ref = self.get_inode_ref(inode);

        // 获取文件大小
        let file_size = inode_ref.inode.size();

        // 如果偏移量大于文件大小，返回 0
        if offset >= file_size as usize {
            return Ok(0);
        }

        // 如果 offset + read_buf_len 大于 file_size，调整读取大小
        if offset + read_buf_len > file_size as usize {
            read_buf_len = file_size as usize - offset;
        }

        // adjust the read buffer size if the read buffer size is greater than the file size
        // 这步是不是和上一步重了？
        let size_to_read = min(read_buf_len, file_size as usize - offset);

        // 计算起始块以及未对齐大小
        let iblock_start = offset / BLOCK_SIZE;
        let iblock_last = (offset + size_to_read + BLOCK_SIZE - 1) / BLOCK_SIZE; // round up to include the last partial block
        let unaligned_start_offset = offset % BLOCK_SIZE;

        // Buffer to keep track of read bytes
        let mut cursor = 0;
        let mut total_bytes_read = 0;
        let mut iblock = iblock_start;

        // Unaligned read at the beginning
        // 处理起始块未对齐的情况
        if unaligned_start_offset > 0 {
            let adjust_read_size = min(BLOCK_SIZE - unaligned_start_offset, size_to_read);

            // 获取逻辑块号对应的物理块号
            let pblock_idx = self.get_pblock_idx(&inode_ref, iblock as u32)?;

            // 读取数据
            let mut data = [0u8; BLOCK_SIZE];
            self.block_device.read_block(pblock_idx as usize, &mut data);

            // 将数据复制到read_buf中
            read_buf[cursor..cursor + adjust_read_size].copy_from_slice(
                &data[unaligned_start_offset..unaligned_start_offset + adjust_read_size],
            );

            // 更新 cursor 以及 total_bytes_read
            cursor += adjust_read_size;
            total_bytes_read += adjust_read_size;
            iblock += 1;
        }

        // Continue with full block reads
        // 继续处理整个的块
        while total_bytes_read < size_to_read {
            let read_length = core::cmp::min(BLOCK_SIZE, size_to_read - total_bytes_read);

            // 获取逻辑块号对应的物理块号
            let pblock_idx = self.get_pblock_idx(&inode_ref, iblock as u32)?;

            // 读取数据
            let mut data = [0u8; BLOCK_SIZE];
            self.block_device.read_block(pblock_idx as usize, &mut data);

            // 将读取到的数据复制到read_buf中
            read_buf[cursor..cursor + read_length].copy_from_slice(&data[..read_length]);

            // 更新 cursor 以及 total_bytes_read
            cursor += read_length;
            total_bytes_read += read_length;
            iblock += 1;
        }

        Ok(min(total_bytes_read, size_to_read))
    }

    /// Write data to a file at a given offset
    ///
    /// Params:
    /// inode: u32 - inode number of the file
    /// offset: usize - offset from where to write
    /// write_buf: &[u8] - buffer to write the data from
    ///
    /// Returns:
    /// `Result<usize>` - number of bytes written
    pub fn write_at(&self, inode: u32, offset: usize, write_buf: &[u8]) -> Result<usize, isize> {
        // write buf is empty, return 0
        let write_buf_len = write_buf.len();
        if write_buf_len == 0 {
            return Ok(0);
        }

        // get the inode reference
        let mut inode_ref = self.get_inode_ref(inode);

        // Get the file size
        let file_size = inode_ref.inode.size();

        // Calculate the start and end block index
        let iblock_start = offset / BLOCK_SIZE;
        let iblock_last = (offset + write_buf_len + BLOCK_SIZE - 1) / BLOCK_SIZE; // round up to include the last partial block

        // start block index
        let mut iblk_idx = iblock_start;
        let ifile_blocks = (file_size + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64;

        // Calculate the unaligned size
        let unaligned = offset % BLOCK_SIZE;

        // Buffer to keep track of written bytes
        let mut written = 0;

        // Start bgid
        let mut start_bgid = 1;

        // Unaligned write
        if unaligned > 0 {
            let len = min(write_buf_len, BLOCK_SIZE - unaligned);
            // Get the physical block id, if the block is not present, append a new block
            let pblock_idx = if iblk_idx < ifile_blocks as usize {
                self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
            } else {
                // physical block not exist, append a new block
                self.append_inode_pblk_from(&mut inode_ref, &mut start_bgid)?
            };

            let mut block =
                Block::load_offset(self.block_device.clone(), pblock_idx as usize * BLOCK_SIZE);

            block.write_offset(unaligned, &write_buf[..len], len);
            block.sync_blk_to_disk(self.block_device.clone());
            drop(block);

            written += len;
            iblk_idx += 1;
        }

        // Aligned write
        let mut fblock_start = 0;
        let mut fblock_count = 0;

        while written < write_buf_len {
            while iblk_idx < iblock_last && written < write_buf_len {
                // Get the physical block id, if the block is not present, append a new block
                let pblock_idx = if iblk_idx < ifile_blocks as usize {
                    self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
                } else {
                    // physical block not exist, append a new block
                    self.append_inode_pblk_from(&mut inode_ref, &mut start_bgid)?
                };
                if fblock_start == 0 {
                    fblock_start = pblock_idx;
                }

                // Check if the block is contiguous
                if fblock_start + fblock_count != pblock_idx {
                    break;
                }

                fblock_count += 1;
                iblk_idx += 1;
            }

            // Write contiguous blocks at once
            let len = min(fblock_count as usize * BLOCK_SIZE, write_buf_len - written);

            for i in 0..fblock_count {
                let block_offset = fblock_start as usize * BLOCK_SIZE + i as usize * BLOCK_SIZE;
                let mut block = Block::load_offset(self.block_device.clone(), block_offset);
                let write_size = min(BLOCK_SIZE, write_buf_len - written);
                block.write_offset(0, &write_buf[written..written + write_size], write_size);
                block.sync_blk_to_disk(self.block_device.clone());
                drop(block);
                written += write_size;
            }

            fblock_start = 0;
            fblock_count = 0;
        }

        // Final unaligned write if any
        if written < write_buf_len {
            let len = write_buf_len - written;
            // Get the physical block id, if the block is not present, append a new block
            let pblock_idx = if iblk_idx < ifile_blocks as usize {
                self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
            } else {
                // physical block not exist, append a new block
                self.append_inode_pblk(&mut inode_ref)?
            };

            let mut block =
                Block::load_offset(self.block_device.clone(), pblock_idx as usize * BLOCK_SIZE);
            block.write_offset(0, &write_buf[written..], len);
            block.sync_blk_to_disk(self.block_device.clone());
            drop(block);

            written += len;
        }

        // Update file size if necessary
        if offset + write_buf_len > file_size as usize {
            log::trace!("set file size {:x}", offset + write_buf_len);
            inode_ref.inode.set_size((offset + write_buf_len) as u64);

            self.write_back_inode(&mut inode_ref);
        }

        Ok(written)
    }

    /// File remove
    ///
    /// Params:
    /// path: file path start from root
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn file_remove(&self, path: &str) -> Result<usize, isize> {
        // start from root
        let mut parent_inode_num = ROOT_INODE;

        let mut nameoff = 0;
        let child_inode = self.generic_open(path, &mut parent_inode_num, false, 0, &mut nameoff)?;

        let mut child_inode_ref = self.get_inode_ref(child_inode);
        let child_link_cnt = child_inode_ref.inode.links_count();
        if child_link_cnt == 1 {
            self.truncate_inode(&mut child_inode_ref, 0)?;
        }

        // get child name
        let mut is_goal = false;
        let p = &path[nameoff as usize..];
        let len = path_check(p, &mut is_goal);

        // load parent
        let mut parent_inode_ref = self.get_inode_ref(parent_inode_num);

        let r = self.unlink(
            &mut parent_inode_ref,
            &mut child_inode_ref,
            &p[..len],
        )?;

        Ok(EOK)
    }

    /// File truncate
    /// + 参数
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// new_size: u64 - 文件的新大小
    /// + 返回值
    /// `Result<usize>` - 操作状态
    pub fn truncate_inode(
        &self,
        inode_ref: &mut Ext4InodeRef,
        new_size: u64,
    ) -> Result<usize, isize> {
        let old_size = inode_ref.inode.size();

        // assert!(old_size > new_size);
        if old_size > new_size{
            println!("[kernel] this may need to be changed");
            return Ok(EOK)
        }

        if old_size == new_size {
            return Ok(EOK);
        }

        let block_size = BLOCK_SIZE as u64;
        let new_blocks_cnt = ((new_size + block_size - 1) / block_size) as u32;
        let old_blocks_cnt = ((old_size + block_size - 1) / block_size) as u32;
        let diff_blocks_cnt = old_blocks_cnt - new_blocks_cnt;

        if diff_blocks_cnt > 0 {
            self.extent_remove_space(inode_ref, new_blocks_cnt, EXT_MAX_BLOCKS)?;
        }

        inode_ref.inode.set_size(new_size);
        self.write_back_inode(inode_ref);

        Ok(EOK)
    }
}

pub struct Ext4FileContentWrapper {
    file_content_inner: RwLock<Ext4FileContent>,
}
