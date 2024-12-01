# 基础环境配置

1. make、Cmake安装（辅助编译工具）
   执行：
   
   ```bash
   sudo pacman -S make cmake
   ```

2. 安装rust对LoongArch的编译链
   
   + 安装rustup（rust的安装器+版本管理器）
   
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   
   + 安装Rust工具链
       由于LoongArch架构的交叉编译Rust工具链已经合并到上游， 目前不需要我们手动安装。在 `Makefile` 中有自动的检测脚本， 只需要后续的make命令即可。
   + 或者
   
   ```bash
   rustup default nightly
   ```
   
   然后用最新的rust-nightly版本编译完成后（编译前还需修改部分代码，包括新添加的naked_asm）会无法运行，并且暂时不知道如何修复，所以使用2024-05-01版本：
   
   ```bash
   rustup default nightly-2024-05-01-x86_64-unknown-linux-gnu
   ```
   
   + 安装交叉编译工具。本项目使用的为在x86_64下编译产生loongarch64的编译工具。
     LoongArch GCC 12:    
     百度网盘链接: https://pan.baidu.com/s/1xHriNdgcNzzn-X9U73sHlw 提取码: 912v   下载完成后，首先将本压缩包解压后，放至`/opt`目录下;
     然后，将本文件夹引入环境变量，在`~/.bashrc`中添加
   
     ```bash
     export PATH="$PATH:/opt/cross-my/bin"
     ```
   
     最后，执行如下命令来更新环境变量。
   
     ```bash
     source ~/.bashrc
     ```
   
     如果是zsh，将上述的内容添加到`~/.zshrc`，然后重新打开终端(shell)或者执行
   
     ```bash
     source ~/.zshrc
     ```
   
3. 缺少部分库文件和编译rust代码出现错误的问题
   建议尝试`make clean`后， 删除对应文件夹的Cargo.lock， 尝试在Cargo.toml中删除版本限制再重新编译。

# 文档信息

+ 目前只有本**README.md**

# 运行方式与运行效果

默认的MODE使用release,因为debug模式会出现panic,目前不知道怎么修复，可能是rustup版本的问题。

```bash
make all
```

# Makefile可用选项相关解释

## 用户程序编译

`make user`: 编译用户程序
`make c-user`: 编译C用户程序
`make rust-user`: 编译用户程序 

## 文件系统编译

`make fat32`: 创建文件系统镜像， 但不刷入虚拟机
`make qemu-flash-fat-img`: 创建文件系统镜像， 且入虚拟机

## 内核编译与运行

注意，在命令后加入LOG=trace可以开启trace及以上的所有log， log从低到高等级分为trace, debug, info, warning, error
`make run`: 编译系统，且执行虚拟机测试
`make runsimple`: 执行虚拟机测试， 但不编译系统
`make gdb`: 执行开启debug模式(需要配合gdb使用)，启动虚拟机但不运行
第一次运行直接`make`即可， 但后续的运行可以直接`make runsimple`, 有时候意外退出或者失败可以考虑使用`make qemu-flash-fat-img`再`make runsimple`

## 其他

`make clean`: 清理已经编译的项目（包括用户程序， 系统和FAT镜像）
