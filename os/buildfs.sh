SUDO=$(if [ $(whoami) = "root" ];then echo -n "";else echo -n "sudo";fi)
U_DIR="../easy-fs-fuse"
U=$1
BLK_SZ="512"
TARGET=riscv64gc-unknown-none-elf
MODE="debug"
# 如果参数数量大于2（实际上在Makefile里面调用这个脚本的时候参数数量是3）
if [ $# -ge 2 ]; then
    if [ "$2"="2k500" -o "$2"="laqemu" ]
    then
        TARGET=loongarch64-unknown-linux-gnu
        BLK_SZ="2048"
    else
        TARGET=$2
    fi
fi

if [ $# -ge 3 ]; then
    MODE="$3"
fi

ARCH=$(echo "${TARGET}" | cut -d- -f1| grep -o '[a-zA-Z]\+[0-9]\+')
echo
echo Current arch: ${ARCH}
echo

mkdir -p ${U_DIR}
touch ${U}
dd if=/dev/zero of=${U} bs=1M count=128
# 如果是fat32文件系统
if [ "$4" = "fat32" ]
then
    echo Making fat32 imgage with BLK_SZ=${BLK_SZ}
    mkfs.vfat -F 32 ${U} -S ${BLK_SZ}
    fdisk -l ${U}
fi

# 如果是ext4文件系统
if [ "$4" = "ext4" ]
then
    echo Making ext4 imgage with BLK_SZ=${BLK_SZ}
    mkfs.ext4 ${U}
    fdisk -l ${U}
fi

if test -e ${U_DIR}/fs
then
    "$SUDO" rm -r ${U_DIR}/fs
fi

"$SUDO" mkdir ${U_DIR}/fs

"$SUDO" mount -f ${U} ${U_DIR}/fs
if [ $? -ne 0 ]
then
    "$SUDO" umount ${U}
fi
"$SUDO" mount ${U} ${U_DIR}/fs

# build root
"$SUDO" mkdir -p ${U_DIR}/fs/lib
# "$SUDO" cp ../user/lib/${ARCH}/libc.so ${U_DIR}/fs/lib
"$SUDO" mkdir -p ${U_DIR}/fs/etc
"$SUDO" mkdir -p ${U_DIR}/fs/bin
"$SUDO" mkdir -p ${U_DIR}/fs/root
"$SUDO" sh -c "echo -e "root:x:0:0:root:/root:/bash\n" > ${U_DIR}/fs/etc/passwd"
"$SUDO" touch ${U_DIR}/fs/root/.bash_history

try_copy(){
    if [ -d $1 ]
    then
        echo copying $1 to $2 ';'
        for programname in $(ls -A $1)
        do
            "$SUDO" cp -fr "$1"/"$programname" $2
        done
    else
        echo "$1" "doesn""'""t exist, skipped."
    fi
}

copy_2024_testcase(){
    echo "copy test case to ${U_DIR}/fs"
    "$SUDO" mkdir -p ${U_DIR}/fs/2024testcase
    "$SUDO" cp -fr ./2024testcase/* ${U_DIR}/fs/2024testcase/
}

for programname in $(ls ../user/src/bin)
do
    "$SUDO" cp -r ../user/target/${TARGET}/${MODE}/${programname%.rs} ${U_DIR}/fs/${programname%.rs}
done

if [ ! -f ${U_DIR}/fs/syscall ]
then
    "$SUDO" mkdir -p ${U_DIR}/fs/syscall
fi

try_copy ../user/user_C_program/user/build/${ARCH}  ${U_DIR}/fs/syscall
try_copy ../user/busybox_lua_testsuites/${ARCH} ${U_DIR}/fs/
copy_2024_testcase
# try_copy ../user/disk/${ARCH} ${U_DIR}/fs/

"$SUDO" umount ${U_DIR}/fs
echo "DONE"
exit 0
