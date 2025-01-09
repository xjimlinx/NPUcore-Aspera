## 写在之前、题目说明

### 题目名称：面向LoongArch指令集的NPUcore ext4文件系统设计与实现

### 题目内容

1. 熟悉Rust语言及其特性，掌握LoongArch指令集，了解文件系统的基本原理和设计方法。
2. 学习NPUcore内核代码，理解其架构及其与文件系统的交互机制。
3. 深入了解ext4文件系统的原理和数据结构，能够参考已有代码设计并实现在NPUcore上的ext4文件系统模块。
4. 对ext4文件系统进行功能测试，包括文件读写、目录管理等，确保功能完整性。
5. 进行性能测试，包括读写速度和延迟等指标，确保满足设计要求。
   作品要求：

1. 实现ext4文件系统的基本功能，包括文件读写、目录管理和权限管理等。
2. 确保ext4文件系统在LoongArch架构下与NPUcore内核无缝兼容，并实现稳定可靠的交互。
3. 实现完整的功能测试和性能测试，确保系统能够处理多种文件操作。
4. 系统稳定性良好，在长时间不间断运行测试中无崩溃现象发生。
    加分内容：
    设计并实现一个虚拟文件系统（VFS）层，将ext4文件系统与NPUcore内核解耦。将开发过程中发现的问题反馈给NPUcore项目组，并贡献修复补丁。

### 目前进度

**实现文件系统实例类型的动态获取，会通过识别文件系统的类型来生成对应文件系统实例**。也就是说直接通过生成根文件系统镜像并传给Qemu后，执行NPUcore就可以识别，相当于VFS层的完善，baseline NPUcore-LA其实VFS层（**如果VFS层的定义没有那么严格的话**）已经将近完成，其VFS只差懒加载的`ROOT`和`FILE_SYSTEM`未完成修改，本仓库在其基础上对`ROOT`和`FILE_SYSTEM`进行了修改，并增加了一个对应的`VFS` Trait，用以完成上述内容。

新的文件系统类型只需要提供实现baseline中提供的`File` Trait，以及`VFS` Trait的接口（并在FS_Type中添加），同时能够适配`BlockDevice` Trait的`write_block`以及`read_block`就应该可以被NPUcore使用。

同时修改了代码中内部的结构（为了帮助自己对代码的理解）。

**（ext4）可以在内核打印信息中打印读取到的超级块、指定块组的描述符、ext4镜像中已经存在的文件内容。**

由于初始化需要对块设备进行写（包括`init_fs`中的创建几个文件夹），ext4下的写操作还未适配好：

+ 当根文件系统镜像是fat32分区时，与baseline所能做的一样

+ 当根文件系统镜像是ext4分区时，还无法进入NPUcore的Bash Shell界面。

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
   
4. Debug编译设置需要调整，因为默认设置无法跳转到动态分发对象内部（即dyn trait对象），在`os/Cargo.toml`内部：

   ```toml
   [profile.dev]
   # 优化级别
   opt-level = "s"
   # debug = true （默认配置）
   # 生成详细调试信息
   debug = 2
   # 禁用调试断言
   debug-assertions = false
   # 整数溢出检查
   overflow-checks = false
   # 禁用链接时优化
   lto = false
   # panic时进行栈展开
   panic = 'unwind'
   # 禁用增量编译
   incremental = false
   # 使用16个代码生成单元
   codegen-units = 16
   # 禁用运行时库搜索路径
   rpath = false
   ```

## 一、运行方式与运行效果

```bash
make all
```

这步会生成根文件系统镜像以及内核镜像

```bash
make gdb
```

此步会启动gdb调试服务，需搭配loongarch的gdb使用

```bash
make run
```

此步会运行qemu同时启动内核、挂载根文件系统镜像，可附加参数`FS_MODE=xxx`，其中`FS_MODE`可为：

+ fat32
+ ext4

## 二、Makefile可用选项相关解释(os目录下)

### 2.0 写在之前

`FS_MODE` 根文件系统镜像类型：

+ `fat32`：默认值
+ `ext4`

> 因为编译的机器是7840HS，全部重新执行编译还是会比较慢，所以有如下的内容

#### 2.0.1 只改动了**rootfs**镜像类型或者user用户态程序

注：更换baseline之后，内核镜像是链接到根文件系统镜像的，所以无法单独分开操作

```bash
# xxx为指定的文件系统类型
make run FS_MODE=xxx
```

#### 2.0.2 只改动了kernel

```bash
make run-inner
```

### 2.1 用户程序编译

`make user`: 编译用户态程序

### 2.2 根文件系统镜像生成

`FS_MODE` 默认为`fat32`，可选项为`ext4`

```bash
# 因为更换了baseline,其qemu启动方式与原来的略有不同，并且rootfs镜像文件是嵌入到内核文件中的
# 所以暂时无法找到单独生成文件系统不处理其他东西的方法，只能先全部编译
make all FS_MODE=xxx
```

### 2.3 内核编译与运行

注意，在命令后加入`LOG=trace`可以开启trace及以上的所有log，
log从低到高等级分为`trace`, `debug`, `info`, `warning`, `error`
`make run`: 编译系统，且启动`qemu`
`make gdb`: 执行开启`debug`模式(需要配合`loongarch64-unknown-linux-gnu-gdb`使用)

## 三、文档信息

### 3.0 写在之前

+ **实践笔记为完成任务思路的展现**

+ 首先，由于本人的惰性以及个人能力有限，所以文档整理的不是很好，可能一份文档两次更新之间会隔很久时间，所以导致在文档中的内容可能会有许多地方不通顺或者说会有比较**低级的错误**（比如说某些概念或者过程的阐述）。
+ 其次，文档中的**Rust笔记**是“笔记”，**主要学习、摘抄来源为菜鸟教程和Rust语言圣经，可能存在部分内容直接Copy的操作（有的是按自己的理解删减了一些内容）**。
+ 再者，有些文档里面会存在**只写了一点点**的情况，或者说有些地方只列出来但是没有写全、缺少补充说明或者定义的情况。
+ ~~然后，还有一些文档只进行了创建，本来想记点东西，但是由于没有写进任何内容所以没有放入Docs中(比如shell脚本笔记以及gdb调试笔记)~~
+ 还有，尝试编译了**qemu**和**gdb**，但是前者需要额外的编译配置才能直接启动NPUcore-LA，在稍微搜罗了一下关于baseline中使用的qemu的信息之后，发现只给了编译后的二进制文件，并没有给编译配置选项，也就是说用最简单的默认配置编译出来的LA64架构的**qemu9**并不能直接使用；对于后者（**gdb**）编译了12以及15.2的版本，在运行的时候都报信息说**架构不匹配**，而在baseline中的`util/qemu-2k500`中使用该版本的qemu时打印的帮助信息是说2k500是mips架构，不知道与这个有没有关系，而为qemu编译MIPS架构的话有多个选项，尝试其中mips64el的编译方式然后也无法正常调试。最终编写了如何编译的文档，但是由于不具有实用性，所以未放在Docs中。

### 3.1 各文档内容说明

+ 本**README.md**: 项目的基本说明
+ [Docs/FS of NPUcore-LA](Docs/FS of NPUcore-LA.md)：NPUcore-LA `fs`模块的注解
+ [Docs/Rust学习笔记](Docs/Rust.md)：NPUcore-LA Rust的学习笔记
+ [Docs/Makefile小记](Docs/Makefile 小记.md)：关于Makefile编写的语法笔记，内容较少
+ [**Docs/实践笔记**](Docs/实践笔记.md)：记录开始真正着手修改NPUcore-LA以适配ext4文件系统的思路和操作过程，由于并不是同时写文档和代码的，所以可能和实际操作会有略微出入，不一定能反映最新的代码修改、添加，但是可以反映整体思路
+ [Docs/Linux文件系统与Ext4原理](Docs/Linux文件系统与Ext4原理.md)：Linux文件系统原理以及Ext4原理以及MBR、GPT的原理，还未写完

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
