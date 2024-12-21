## 〇、基础环境配置

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

   然后用最新的rust-nightly版本编译完成后（编译前还需修改部分代码，包括新添加的naked_asm）会无法运行，并且暂时不知道如何修复，所以使用`2024-05-01`版本：

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
   建议尝试`make clean`后， 删除对应文件夹的`Cargo.lock`， 尝试在`Cargo.toml`中删除版本限制再重新编译。

## 一、运行方式与运行效果

默认的MODE使用`release`,因为`debug`模式会出现`panic`,目前不知道怎么修复，可能是`rust`版本的问题。

```bash
make all
```

这步会清理所有生成的内容并执行编译

## 二、Makefile可用选项相关解释(os目录下)

### 2.0 写在之前

> 因为编译的机器是7840HS，全部重新执行编译还是会比较慢，所以有如下的内容

#### 2.0.1 只改动了**rootfs**镜像类型

```bash
# xxx为指定的文件系统类型
make remake-qemu-flash-img FS_MODE=xxx
# 然后执行
make runsimple
```

#### 2.0.2 只改动了kernel

```bash
# 此步编译内核并链接到fs-img-dir目录下的uImage
make build
# 然后执行
make runsimple
```

#### 2.0.3 只改动用户程序

因为实际上也是改动根文件系统，所以与 [2.0.1](####2.0.1 只改动了rootfs镜像类型) 相同

### 2.1 用户程序编译

`make user`: 编译用户程序
`make c-user`: 编译 C 用户程序
`make rust-user`: 编译 Rust 用户程序

### 2.2 文件系统编译

`make fat32`: 创建文件系统镜像， 但不写入qemu使用的nand.dat
`make qemu-flash-fat-img`: 创建文件系统镜像， 且写入qemu使用的nand.dat

若需要进行不同文件系统的测试，先进行：
`make remake-qemu-flash-img FS_MODE=xxx`: xxx=ext4或者fat32，这样会为qemu生成指定的根文件系统镜像
再执行：
`make runsimple`: 运行qemu

### 2.3 内核编译与运行

注意，在命令后加入`LOG=trace`可以开启trace及以上的所有log，
log从低到高等级分为`trace`, `debug`, `info`, `warning`, `error`
`make run`: 编译系统，且执行虚拟机测试
`make runsimple`: 执行虚拟机测试， 但不编译系统
`make gdb`: 执行开启debug模式(需要配合gdb使用)

## 三、文档信息

+ 目前只有本**README.md**
