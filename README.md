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

或者直接执行

```bash
# 阅读Makefile可以发现此步调用 build 和 do-run
# 而 runsimple 也只执行 do-run
make run-inner
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

### 3.0 写在之前

+ 首先，由于本人的惰性以及个人能力有限，所以文档整理的不是很好，可能一份文档两次更新之间会隔很久时间，所以导致在文档中的内容可能会有许多地方不通顺或者说会有比较**低级的错误**（比如说某些概念或者过程的阐述）。
+ 其次，文档中的**Rust笔记**是“笔记”，**主要学习、摘抄来源为菜鸟教程和Rust语言圣经，可能存在部分内容直接Copy的操作（有的是按自己的理解删减了一些内容）**。
+ 再者，有些文档里面会存在**只写了一点点**的情况，或者说有些地方只列出来但是没有写全、缺少补充说明或者定义的情况。
+ ~~然后，还有一些文档只进行了创建，本来想记点东西，但是由于没有写进任何内容所以没有放入Docs中(比如shell脚本笔记以及gdb调试笔记)~~
+ 还有，尝试编译了**qemu**和**gdb**，但是前者需要额外的编译配置才能直接启动NPUcore-LA，在稍微搜罗了一下关于baseline中使用的qemu的信息之后，发现只给了编译后的二进制文件，并没有给编译配置选项，也就是说用最简单的默认配置编译出来的LA64架构的**qemu9**并不能直接使用；对于后者（**gdb**）编译了12以及15.2的版本，在运行的时候都报信息说**架构不匹配**，而在baseline中的util/qemu-2k500中使用该版本的qemu时打印的帮助信息是说2k500是mips架构，不知道与这个有没有关系，而为qemu编译MIPS架构的话有多个选项，尝试其中mips64el的编译方式然后也无法正常调试。最终编写了如何编译的文档，但是由于不具有实用性，所以未放在Docs中。

### 3.1 各文档内容说明

+ 本**README.md**: 项目的基本说明
+ [Docs/FS of NPUcore-LA](Docs/FS of NPUcore-LA.md)：NPUcore-LA `fs`模块的注解
+ [Docs/Rust学习笔记](Docs/Rust.md)：NPUcore-LA Rust的学习笔记
+ [Docs/Makefile小记](Docs/Makefile 小记.md)：关于Makefile编写的语法笔记，内容较少
+ [Docs/实践笔记](Docs/实践笔记.md)：记录开始真正着手修改NPUcore-LA以适配ext4文件系统的思路和操作过程，由于并不是同时写文档和代码的，所以可能和实际操作会有略微出入，不一定能反映最新的代码修改、添加，但是可以反映整体思路
+ [Docs/Linux文件系统与Ext4原理](Docs/Linux文件系统与Ext4原理)：Linux文件系统原理以及Ext4原理以及MBR、GPT的原理，还未写完

## 附录、参考来源（主要参考来源）

### 附录、写在之前

1-2 为rust的教程，NPUcore使用的是Rust语言，所以必须先学习Rust

3-4 为fat32原理，都是b站视频

5-6 选取了看过的关于 ext4 的对我来说比较有帮助的介绍、原理文章或视频，其中 5 是文章，6 是视频

其余还有部分内容会零散出现在各个文档里面。

Rust教程

[1] [菜鸟教程 Rust教程](https://www.runoob.com/rust/rust-tutorial.html)

[2] [Rust 语言圣经](https://course.rs/about-book.html)

fat32原理

[3] [「Coding Master」第24话 FAT32文件系统？盘它！](https://www.bilibili.com/video/BV1L64y1o74u/)

[4] [基于表的文件系统：FAT [中山大学 操作系统原理]](https://www.bilibili.com/video/BV18x4y1y7Qs)

ext4原理

[5] [14.ext2文件系统](https://www.bilibili.com/video/BV1V84y1A7or)

[6] [第4章 ext文件系统机制原理剖析](https://www.cnblogs.com/f-ck-need-u/p/7016077.html)
