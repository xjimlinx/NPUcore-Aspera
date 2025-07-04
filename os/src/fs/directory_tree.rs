use super::inode::DiskInodeType;
use super::vfs::VFS;
use super::{
    cache::BlockCacheManager,
    dev::{null::Null, tty::Teletype, zero::Zero},
    file_trait::File,
    filesystem::FileSystem,
    layout::OpenFlags,
    Hwclock,
};
use crate::fs::dev::urandom::Urandom;
use crate::fs::fat32::FatOSInode;
#[cfg(feature = "oom_handler")]
use crate::mm::tlb_invalidate;
use crate::syscall::errno::*;
use crate::{drivers::BLOCK_DEVICE, fs::filesystem::FS_Type};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use lazy_static::*;
use spin::{Mutex, MutexGuard, RwLock, RwLockWriteGuard};

lazy_static! {
    // 文件系统实例
    pub static ref FILE_SYSTEM: Arc<dyn VFS> =
        <dyn VFS>::open_fs(BLOCK_DEVICE.clone(), Arc::new(Mutex::new(BlockCacheManager::new())));
    // 目录树根节点
    pub static ref ROOT: Arc<DirectoryTreeNode> = {
        let curr_fs_type = FILE_SYSTEM.get_filesystem_type();
        let inode = DirectoryTreeNode::new(
            // 因为是根节点，所以没有名字（根目录是不是只有‘/’，斜杠左边是不是啥也没有？）
            "".to_string(),
            // 通过获取FILE_SYSTEM的类型来创建目录树的文件系统字段
            Arc::new(FileSystem::new(curr_fs_type)),
            // // 系统Inode，包装了具体文件系统的Inode
            // OSInode::new(<dyn VFS>::root_inode(&FILE_SYSTEM)),
            <dyn VFS>::root_osinode(&FILE_SYSTEM),
            // 父节点，因为是根节点所以没有父节点
            Weak::new(),
        );
        inode.add_special_use();
        inode
    };
    static ref DIRECTORY_VEC: Mutex<(Vec<Weak<DirectoryTreeNode>>, usize)> =
        Mutex::new((Vec::new(), 0));
    static ref PATH_CACHE: Mutex<(String, Weak<DirectoryTreeNode>)> =
        Mutex::new(("".to_string(), Weak::new()));
}

// 插入一个节点到 DIRECTORY_VEC 中
fn insert_directory_vec(inode: Weak<DirectoryTreeNode>) {
    DIRECTORY_VEC.lock().0.push(inode);
}

// 删除一个节点
// 每次触发 Drop 特征时，都会调用这个函数
fn delete_directory_vec() {
    let mut lock = DIRECTORY_VEC.lock();
    // 增加计数器的值
    lock.1 += 1;
    // 如果计数器的值大于等于 DIRECTORY_VEC中节点数的一半，更新 DIRECTORY_VEC
    if lock.1 >= lock.0.len() / 2 {
        update_directory_vec(&mut lock);
    }
}

// 优化 DIRECTORY_VEC，在计数器达到节点数一半时触发
fn update_directory_vec(lock: &mut MutexGuard<(Vec<Weak<DirectoryTreeNode>>, usize)>) {
    // 初始化一个新的 DIRECTORY_VEC
    let mut new_vec: Vec<Weak<DirectoryTreeNode>> = Vec::new();
    // 遍历原来的 DIRECTORY_VEC，如果 inode 存在，添加到新的 DIRECTORY_VEC中
    for inode in &lock.0 {
        if inode.upgrade().is_some() {
            new_vec.push(inode.clone());
        }
    }
    // 赋值给 DIRECTORY_VEC，同时将计数器置为0
    **lock = (new_vec, 0);
}

pub struct DirectoryTreeNode {
    /// 如果这是个目录
    /// 1. cwd 当前工作目录
    /// 2. mount point 挂载点
    /// 3. root node 根节点
    /// 如果这是个文件
    /// 1. 被某些进程执行
    /// 该参数在打开时增加1
    spe_usage: Mutex<usize>,
    pub name: String,
    // 文件系统实例
    filesystem: Arc<FileSystem>,
    // 文件
    pub file: Arc<dyn File>,
    // 指向自己的弱引用
    selfptr: Mutex<Weak<Self>>,
    // 指向父节点的弱引用
    father: Mutex<Weak<Self>>,
    // 子节点
    children: RwLock<Option<BTreeMap<String, Arc<Self>>>>,
}

// 实现 Drop 特征，当一个 DirectoryTreeNode 被销毁时，会调用 delete_directory_vec 函数
impl Drop for DirectoryTreeNode {
    fn drop(&mut self) {
        delete_directory_vec();
    }
}

impl DirectoryTreeNode {
    pub fn new(
        name: String,
        filesystem: Arc<FileSystem>,
        file: Arc<dyn File>,
        father: Weak<Self>,
    ) -> Arc<Self> {
        let node = Arc::new(DirectoryTreeNode {
            // 初始化为0
            spe_usage: Mutex::new(0),
            name,
            filesystem,
            file,
            selfptr: Mutex::new(Weak::new()),
            father: Mutex::new(father),
            // 子节点初始化为 None
            children: RwLock::new(None),
        });
        *node.selfptr.lock() = Arc::downgrade(&node);
        node.file.info_dirtree_node(Arc::downgrade(&node));
        insert_directory_vec(Arc::downgrade(&node));
        node
    }

    pub fn add_special_use(&self) {
        *self.spe_usage.lock() += 1;
    }

    pub fn sub_special_use(&self) {
        *self.spe_usage.lock() -= 1;
    }

    // 获取当前工作目录，返回一个 String 类型（绝对路径）
    pub fn get_cwd(&self) -> String {
        // 创建一个pathv变量，最多容量为8（个String变量）,
        let mut pathv = Vec::<String>::with_capacity(8);
        // 循环获取父节点，直到根节点为止，并将每一级的节点名称添加到pathv中
        let mut current_inode = self.get_arc();
        loop {
            let lock = current_inode.father.lock();
            let par_inode = match lock.upgrade() {
                Some(inode) => inode.clone(),
                None => break,
            };
            drop(lock);
            pathv.push(current_inode.name.clone());
            current_inode = par_inode;
        }
        pathv.push(current_inode.name.clone());
        pathv.reverse();
        if pathv.len() == 1 {
            "/".to_string()
        } else {
            pathv.join("/")
        }
    }

    // 获取自身的强引用，upgrade() 方法返回一个 Option<Arc<T>> 类型
    fn get_arc(&self) -> Arc<Self> {
        self.selfptr.lock().upgrade().unwrap().clone()
    }

    /// 解析路径
    /// # 参数
    /// + path: 路径
    /// # 返回值
    /// + 一个 Vec<&str> 类型，存储路径的每一级目录
    /// # 说明
    /// 比如路径是“/lib/a/.././d/c”
    /// 那么存入的内容就是
    /// ["a", "d", "c"]
    fn parse_dir_path(path: &str) -> Vec<&str> {
        path.split('/').fold(Vec::with_capacity(8), |mut v, s| {
            match s {
                // 去掉空字符串和当前目录
                "" | "." => {}
                ".." => {
                    if v.last().map_or(true, |s| *s == "..") {
                        v.push(s);
                    } else {
                        v.pop();
                    }
                }
                _ => {
                    v.push(s);
                }
            }
            v
        })
    }

    // 缓存该文件夹下的所有子文件到lock中
    fn cache_all_subfile(
        &self,
        lock: &mut RwLockWriteGuard<Option<BTreeMap<String, Arc<Self>>>>,
    ) -> Result<(), isize> {
        if lock.is_some() {
            return Ok(());
        }
        if !self.file.is_dir() {
            return Err(ENOTDIR);
        }
        let vec = match self.file.open_subfile() {
            Ok(vec) => vec,
            Err(errno) => return Err(errno),
        };
        let mut map = BTreeMap::new();
        for (name, file) in vec {
            let key = name.clone();
            let value = Self::new(
                key.clone(),
                self.filesystem.clone(),
                file.clone(),
                Arc::downgrade(&self.get_arc()),
            );
            map.insert(key, value);
        }
        **lock = Some(map);
        Ok(())
    }

    // 尝试获取子文件
    fn try_to_open_subfile(
        &self,
        name: &str,
        lock: &mut RwLockWriteGuard<Option<BTreeMap<String, Arc<Self>>>>,
    ) -> Result<Arc<Self>, isize> {
        match self.cache_all_subfile(lock) {
            Ok(_) => {}
            Err(errno) => return Err(errno),
        };
        match lock.as_ref().unwrap().get(&name.to_string()) {
            Some(child) => Ok(child.clone()),
            None => Err(ENOENT),
        }
    }

    // 通过一个动态数组 components 来进入某个目录
    pub fn cd_comp(&self, components: &Vec<&str>) -> Result<Arc<Self>, isize> {
        let mut current_inode = self.get_arc();
        for component in components {
            if *component == ".." {
                let lock = current_inode.father.lock();
                let par_inode = lock.upgrade();
                match par_inode {
                    Some(par_inode) => {
                        drop(lock);
                        current_inode = par_inode;
                    }
                    None => {}
                }
                continue;
            }
            let mut lock = current_inode.children.write();
            match current_inode.try_to_open_subfile(component, &mut lock) {
                Ok(child_inode) => {
                    let child_inode = child_inode.clone();
                    drop(lock);
                    current_inode = child_inode.clone()
                }
                Err(errno) => return Err(errno),
            }
        }
        Ok(current_inode)
    }
    // 调用 cd_comp 方法，通过一个字符串 path 来进入某个目录
    // 其中 path 会调用 parse_dir_path 方法来解析
    pub fn cd_path(&self, path: &str) -> Result<Arc<Self>, isize> {
        let components = Self::parse_dir_path(path);
        let inode = if path.starts_with("/") {
            &**ROOT
        } else {
            &self
        };
        inode.cd_comp(&components)
    }

    // 创建一个子文件，文件名和文件类型由参数提供
    // file_type: 文件是常规文件还是目录
    fn create(&self, name: &str, file_type: DiskInodeType) -> Result<Arc<dyn File>, isize> {
        // if name == "" || !self.file.is_dir() {
        //     debug_assert!(false);
        // }
        self.file.create(name, file_type)
    }

    // 模拟文件系统的 open 调用
    pub fn open(
        &self,
        path: &str,
        flags: OpenFlags,
        special_use: bool,
    ) -> Result<Arc<dyn File>, isize> {
        log::debug!("[open]: cwd: {}, path: {}", self.get_cwd(), path);

        const BUSYBOX_PATH: &str = "/busybox";
        const REDIRECT_TO_BUSYBOX: [&str; 4] = ["/touch", "/rm", "/ls", "/grep"];
        let path = if REDIRECT_TO_BUSYBOX.contains(&path) {
            BUSYBOX_PATH
        } else {
            path
        };
        const LIBC_PATH: &str = "/lib/libc.so";
        const REDIRECT_TO_LIBC: [&str; 3] = [
            "/lib/ld-musl-riscv64.so.1",
            "/lib/ld-musl-riscv64-sf.so.1",
            "/lib/ld-linux-riscv64-lp64d.so.1",
        ];
        let path = if REDIRECT_TO_LIBC.contains(&path) {
            LIBC_PATH
        } else {
            path
        };
        let path = if path == "/usr/lib/tls_get_new-dtv_dso.so" {
            "./libtls_get_new-dtv_dso.so"
        } else {
            path
        };
        // 获取目录树根节点
        let inode = if path.starts_with("/") {
            &**ROOT
        } else {
            &self
        };

        // 获取路径缓存
        let mut path_cache_lock = PATH_CACHE.lock();
        // 如果路径以 '/' 开头，且路径等于缓存路径，且缓存路径的弱引用存在
        let inode = if path.starts_with('/')
            && path == path_cache_lock.0
            && path_cache_lock.1.upgrade().is_some()
        {
            // 获取缓存路径的弱引用
            path_cache_lock.1.upgrade().unwrap()
        } else {
            // 解析路径
            let mut components = Self::parse_dir_path(path);
            // 获取目录栈的栈顶，也就是父目录或者文件本身
            let last_comp = components.pop();
            // 从剩余的路径中获取父目录节点
            let inode = match inode.cd_comp(&components) {
                Ok(inode) => inode,
                Err(errno) => return Err(errno),
            };
            // 若最后一个组件存在，则进行处理
            if let Some(last_comp) = last_comp {
                let mut lock = inode.children.write();
                match inode.try_to_open_subfile(last_comp, &mut lock) {
                    Ok(inode) => {
                        if flags.contains(OpenFlags::O_CREAT | OpenFlags::O_EXCL) {
                            return Err(EEXIST);
                        }
                        inode
                    }
                    Err(ENOENT) => {
                        if !flags.contains(OpenFlags::O_CREAT) {
                            return Err(ENOENT);
                        }
                        let new_file = match inode.create(last_comp, DiskInodeType::File) {
                            Ok(file) => file,
                            Err(errno) => return Err(errno),
                        };
                        let key = (*last_comp).to_string();
                        let value = Self::new(
                            key.clone(),
                            inode.filesystem.clone(),
                            new_file,
                            Arc::downgrade(&inode.get_arc()),
                        );
                        let new_inode = value.clone();
                        lock.as_mut().unwrap().insert(key, value);
                        new_inode
                    }
                    Err(errno) => {
                        return Err(errno);
                    }
                }
            } else {
                inode
            }
        };

        if flags.contains(OpenFlags::O_TRUNC) {
            match inode.file.truncate_size(0) {
                Ok(_) => {}
                Err(errno) => return Err(errno),
            }
        }

        if inode.file.is_file()
            && *inode.spe_usage.lock() > 0
            && (flags.contains(OpenFlags::O_WRONLY) || flags.contains(OpenFlags::O_RDWR))
        {
            return Err(ETXTBSY);
        }

        if inode.file.is_dir()
            && (flags.contains(OpenFlags::O_WRONLY) || flags.contains(OpenFlags::O_RDWR))
        {
            return Err(EISDIR);
        }

        if !inode.file.is_dir() && flags.contains(OpenFlags::O_DIRECTORY) {
            return Err(ENOTDIR);
        }

        if special_use {
            *inode.spe_usage.lock() += 1;
        }

        if path.starts_with('/') && path != path_cache_lock.0 {
            *path_cache_lock = (path.to_string(), Arc::downgrade(&inode.get_arc()));
        }

        Ok(inode.file.open(flags, special_use))
    }

    // 创建一个文件夹
    pub fn mkdir(&self, path: &str) -> Result<(), isize> {
        let inode = if path.starts_with("/") {
            &**ROOT
        } else {
            &self
        };

        let mut components = Self::parse_dir_path(path);
        let last_comp = components.pop();
        let inode = match inode.cd_comp(&components) {
            Ok(inode) => inode,
            Err(errno) => return Err(errno),
        };

        if let Some(last_comp) = last_comp {
            let mut lock = inode.children.write();
            match inode.try_to_open_subfile(last_comp, &mut lock) {
                Ok(_) => {
                    return Err(EEXIST);
                }
                Err(ENOENT) => {
                    let new_file = match inode.create(last_comp, DiskInodeType::Directory) {
                        Ok(file) => file,
                        Err(errno) => return Err(errno),
                    };
                    let key = (*last_comp).to_string();
                    let value = Self::new(
                        key.clone(),
                        inode.filesystem.clone(),
                        new_file,
                        Arc::downgrade(&inode.get_arc()),
                    );
                    let new_inode = value.clone();
                    lock.as_mut().unwrap().insert(key, value);
                    new_inode
                }
                Err(errno) => {
                    return Err(errno);
                }
            }
        } else {
            return Err(EEXIST);
        };

        Ok(())
    }

    // 删除一个文件夹或文件
    pub fn delete(&self, path: &str, delete_directory: bool) -> Result<(), isize> {
        if path.split('/').last().map_or(true, |x| x == ".") {
            return Err(EINVAL);
        }

        let inode = if path.starts_with("/") {
            &**ROOT
        } else {
            &self
        };

        let components = Self::parse_dir_path(path);
        let last_comp = *components.last().unwrap();
        let inode = match inode.cd_comp(&components) {
            Ok(inode) => inode,
            Err(errno) => return Err(errno),
        };

        if *inode.spe_usage.lock() > 0 {
            return Err(EBUSY);
        }

        if !delete_directory && inode.file.is_dir() {
            return Err(EISDIR);
        }

        if delete_directory && !inode.file.is_dir() {
            return Err(ENOTDIR);
        }

        match inode.father.lock().upgrade() {
            Some(par_inode) => {
                let mut lock = par_inode.children.write();
                match inode.file.unlink(true) {
                    Ok(_) => {
                        let key = last_comp.to_string();
                        lock.as_mut().unwrap().remove(&key);
                    }
                    Err(errno) => return Err(errno),
                }
            }
            None => return Err(EACCES),
        }
        Ok(())
    }

    // 重命名一个文件夹或文件
    pub fn rename(old_path: &str, new_path: &str) -> Result<(), isize> {
        assert!(old_path.starts_with('/'));
        assert!(new_path.starts_with('/'));

        let mut old_comps = Self::parse_dir_path(old_path);
        let mut new_comps = Self::parse_dir_path(new_path);

        if old_comps == new_comps {
            return Ok(());
        }

        if new_comps.starts_with(&old_comps) {
            return Err(EINVAL);
        }
        // We gurantee that last component isn't empty
        let old_last_comp = old_comps.pop().unwrap();
        let new_last_comp = new_comps.pop().unwrap();

        let old_par_inode = match ROOT.cd_comp(&old_comps) {
            Ok(inode) => inode,
            Err(errno) => return Err(errno),
        };
        let new_par_inode = match ROOT.cd_comp(&new_comps) {
            Ok(inode) => inode,
            Err(errno) => return Err(errno),
        };
        type ChildLockType<'a> =
            RwLockWriteGuard<'a, Option<BTreeMap<String, Arc<DirectoryTreeNode>>>>;

        let old_lock: Arc<Mutex<ChildLockType<'_>>>;
        let new_lock: Arc<Mutex<ChildLockType<'_>>>;

        // Be careful about the lock ordering
        if old_comps == new_comps {
            old_lock = Arc::new(Mutex::new(old_par_inode.children.write()));
            new_lock = old_lock.clone();
        } else if old_comps < new_comps {
            old_lock = Arc::new(Mutex::new(old_par_inode.children.write()));
            new_lock = Arc::new(Mutex::new(new_par_inode.children.write()));
        } else {
            new_lock = Arc::new(Mutex::new(new_par_inode.children.write()));
            old_lock = Arc::new(Mutex::new(old_par_inode.children.write()));
        }

        let old_inode =
            match old_par_inode.try_to_open_subfile(old_last_comp, &mut (*old_lock.lock())) {
                Ok(inode) => inode,
                Err(errno) => return Err(errno),
            };

        if *old_inode.spe_usage.lock() > 0 {
            return Err(EBUSY);
        }

        if old_inode.filesystem.fs_id != new_par_inode.filesystem.fs_id {
            return Err(EXDEV);
        }
        let old_key = old_last_comp.to_string();
        let new_key = new_last_comp.to_string();
        match new_par_inode.try_to_open_subfile(new_last_comp, &mut (*new_lock.lock())) {
            Ok(new_inode) => {
                if new_inode.file.is_dir() && !old_inode.file.is_dir() {
                    return Err(EISDIR);
                }
                if old_inode.file.is_dir() && !new_inode.file.is_dir() {
                    return Err(ENOTDIR);
                }
                if *new_inode.spe_usage.lock() > 0 {
                    return Err(EBUSY);
                }
                // delete
                match new_par_inode.file.unlink(true) {
                    Ok(_) => {
                        new_lock.lock().as_mut().unwrap().remove(&new_key);
                    }
                    Err(errno) => return Err(errno),
                }
            }
            Err(ENOENT) => {}
            Err(errno) => return Err(errno),
        }

        let value = old_lock.lock().as_mut().unwrap().remove(&old_key).unwrap();
        match old_inode.file.unlink(false) {
            Ok(_) => {}
            Err(errno) => return Err(errno),
        };
        match old_inode.filesystem.fs_type {
            FS_Type::Fat32 => {
                let old_file = old_inode.file.downcast_ref::<FatOSInode>().unwrap();
                let new_par_file = new_par_inode.file.downcast_ref::<FatOSInode>().unwrap();
                new_par_file.link_child(old_last_comp, old_file)?;
            }
            FS_Type::Ext4 => {
                todo!()
            }
            FS_Type::Null => return Err(EACCES),
        }
        *value.father.lock() = Arc::downgrade(&new_par_inode.get_arc());
        new_lock.lock().as_mut().unwrap().insert(new_key, value);

        Ok(())
    }
}

// 用于处理OOM的情况，被 mm 模块调用
// 会调用 tlb_invalidate 函数，在 arch/la64中实现
// 会调用 update_directory_vec 函数
#[cfg(feature = "oom_handler")]
pub fn oom() -> usize {
    tlb_invalidate();
    const MAX_FAIL_TIME: usize = 3;
    let mut fail_time = 0;
    log::warn!("[oom] start oom");
    let mut lock = DIRECTORY_VEC.lock();
    update_directory_vec(&mut lock);
    loop {
        let mut dropped = 0;
        for inode in &lock.0 {
            let inode = inode.upgrade().unwrap();
            dropped += inode.file.oom();
        }
        if dropped > 0 {
            log::warn!("[oom] recycle pages: {}", dropped);
            return dropped;
        }
        fail_time += 1;
        if fail_time >= MAX_FAIL_TIME {
            return dropped;
        }
    }
}

// 初始化文件系统
pub fn init_fs() {
    init_device_directory();
    init_tmp_directory();
    init_proc_directory();
}
#[allow(unused)]
// 初始化设备目录
fn init_device_directory() {
    ROOT.mkdir("/dev");

    let dev_inode = match ROOT.cd_path("/dev") {
        Ok(inode) => inode,
        Err(_) => panic!("dev directory doesn't exist"),
    };

    println!("[kernel] /dev init Successfully!");

    dev_inode.mkdir("shm");
    dev_inode.mkdir("misc");

    println!("[kernel] shm and misc init Successfully!");

    let null_dev = DirectoryTreeNode::new(
        "null".to_string(),
        Arc::new(FileSystem::new(FS_Type::Null)),
        Arc::new(Null {}),
        Arc::downgrade(&dev_inode.get_arc()),
    );
    println!("[kernel] null_dev init successfully!");
    let zero_dev = DirectoryTreeNode::new(
        "zero".to_string(),
        Arc::new(FileSystem::new(FS_Type::Null)),
        Arc::new(Zero {}),
        Arc::downgrade(&dev_inode.get_arc()),
    );
    println!("[kernel] zero_dev init successfully!");
    let urandom_dev = DirectoryTreeNode::new(
        "urandom".to_string(),
        Arc::new(FileSystem::new(FS_Type::Null)),
        Arc::new(Urandom {}),
        Arc::downgrade(&dev_inode.get_arc()),
    );
    println!("[kernel] urandom_dev init successfully!");
    let tty_dev = DirectoryTreeNode::new(
        "tty".to_string(),
        Arc::new(FileSystem::new(FS_Type::Null)),
        Arc::new(Teletype::new()),
        Arc::downgrade(&dev_inode.get_arc()),
    );

    println!("[kernel] tty_dev init successfully!");
    let mut lock = dev_inode.children.write();
    lock.as_mut().unwrap().insert("null".to_string(), null_dev);
    lock.as_mut().unwrap().insert("zero".to_string(), zero_dev);
    lock.as_mut().unwrap().insert("tty".to_string(), tty_dev);
    drop(lock);

    let misc_inode = match dev_inode.cd_path("./misc") {
        Ok(inode) => inode,
        Err(_) => panic!("misc directory doesn't exist"),
    };
    let hwclock_dev = DirectoryTreeNode::new(
        "rtc".to_string(),
        Arc::new(FileSystem::new(FS_Type::Null)),
        Arc::new(Hwclock {}),
        Arc::downgrade(&misc_inode.get_arc()),
    );
    let mut lock = misc_inode.children.write();
    misc_inode.cache_all_subfile(&mut lock);
    lock.as_mut()
        .unwrap()
        .insert("rtc".to_string(), hwclock_dev);
    drop(lock);
}
// 初始化临时文件目录
fn init_tmp_directory() {
    match ROOT.mkdir("/tmp") {
        _ => {}
    }
    println!("[kernel] init_tmp_directory successfully!");
}
// 初始化进程目录
fn init_proc_directory() {
    match ROOT.mkdir("/proc") {
        _ => {}
    }
    println!("[kernel] init_proc_directory successfully!");
    match ROOT.open("/proc/meminfo", OpenFlags::O_CREAT, false) {
        _ => {}
    }
    println!("[kernel] init_proc_meminfo_directory successfully!");
    match ROOT.open("/proc/mounts", OpenFlags::O_CREAT, false) {
        _ => {}
    }
    println!("[kernel] init_proc_mounts_directory successfully!");
}
