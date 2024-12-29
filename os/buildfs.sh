SUDO=$(if [ $(whoami) = "root" ]; then echo -n ""; else echo -n "sudo"; fi)
U_FS_DIR="../fs-img-dir"
U_FS=$1
BLK_SZ="512"
TARGET=riscv64gc-unknown-none-elf
MODE="release"
if [ $# -ge 2 ]; then
    if [ "$2"="2k1000" -o "$2"="laqemu" ]; then
        TARGET=loongarch64-unknown-linux-gnu
        BLK_SZ="2048"
    else
        TARGET=$2
    fi
fi

if [ $# -ge 3 ]; then
    MODE="$3"
fi

ARCH=$(echo "${TARGET}" | cut -d- -f1 | grep -o '[a-zA-Z]\+[0-9]\+')
echo
echo Current arch: ${ARCH}
echo

mkdir -p ${U_FS_DIR}
touch ${U_FS}
dd if=/dev/zero of=${U_FS} bs=1M count=56

if [ "$4" = "fat32" ]; then
    echo Making fat32 imgage with BLK_SZ=${BLK_SZ}
    mkfs.vfat -F 32 ${U_FS} -S ${BLK_SZ}
    fdisk -l ${U_FS}
fi

if [ "$4" = "ext4" ]; then
    echo Making ext4 imgage with BLK_SZ=${BLK_SZ}
    mkfs.ext4 ${U_FS} -b ${BLK_SZ}
    fdisk -l ${U_FS}
fi

if test -e ${U_FS_DIR}/fs; then
    "$SUDO" rm -r ${U_FS_DIR}/fs
fi

"$SUDO" mkdir ${U_FS_DIR}/fs

"$SUDO" mount -f ${U_FS} ${U_FS_DIR}/fs
if [ $? -ne 0 ]; then
    "$SUDO" umount ${U_FS}
fi
"$SUDO" mount ${U_FS} ${U_FS_DIR}/fs

# 创建根文件系统
"$SUDO" mkdir -p ${U_FS_DIR}/fs/lib
"$SUDO" mkdir -p ${U_FS_DIR}/fs/etc
"$SUDO" mkdir -p ${U_FS_DIR}/fs/bin
"$SUDO" mkdir -p ${U_FS_DIR}/fs/root
"$SUDO" sh -c "echo -e "root:x:0:0:root:/root:/bash\n" > ${U_FS_DIR}/fs/etc/passwd"
"$SUDO" touch ${U_FS_DIR}/fs/root/.bash_history

# 只能copy一个文件夹下所有内容，无法copy单文件
try_copy() {
    if [ -d $1 ]; then
        echo copying $1 ';'
        for programname in $(ls -A $1); do
            "$SUDO" cp -fr "$1"/"$programname" $2
        done
    else
        echo "$1" "doesn""'""t exist, skipped."
    fi
}

for programname in $(ls ../user/src/bin); do
    "$SUDO" cp -r ../user/target/${TARGET}/${MODE}/${programname%.rs} ${U_FS_DIR}/fs/${programname%.rs}
done

if [ ! -f ${U_FS_DIR}/fs/syscall ]; then
    "$SUDO" mkdir -p ${U_FS_DIR}/fs/syscall
fi

try_copy ../user/busybox_lua_testsuites/${ARCH} ${U_FS_DIR}/fs/
try_copy ../user/fs ${U_FS_DIR}/fs/
try_copy ../live/splice-test ${U_FS_DIR}/fs/

"$SUDO" umount ${U_FS_DIR}/fs
echo "DONE"
exit 0