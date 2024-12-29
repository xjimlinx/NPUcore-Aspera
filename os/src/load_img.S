    .section .data
    .global simg
    .global eimg
    .align 12
simg:
    .incbin "../fs-img-dir/rootfs-ubifs-ze.img"
eimg:
    .align 12