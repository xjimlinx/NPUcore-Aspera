    # 定义数据段，用于存储文件系统镜像
    .section .data
    # 声明两个全局符号，分别表示镜像起始地址和结束地址
    .global simg
    .global eimg
    # 将数据对齐到2^12次方也就是4096B
    .align 12
# 文件系统镜像起始地址
simg:
    # 将二进制文件包含到当前文件中
    .incbin "../fs-img-dir/rootfs-rv.img"
# 文件系统镜像结束地址
eimg:
    # 再次将数据对齐到4KB边界
    .align 12
