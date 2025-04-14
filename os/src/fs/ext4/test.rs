use alloc::{string::String, vec};

use super::ext4fs::Ext4FileSystem;

impl Ext4FileSystem {
    /// 尝试打开一个文件并读取内容
    /// 读取 2048 个字节
    pub fn test_get_file(&self, path: &str) {
        let read_size = 2048;
        let child_inode = self.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
        println!("child_inode_num: {:?}", child_inode);
        let mut data = vec![0u8; read_size as usize];
        // 读取文件内容
        let bytes_read = self.read_at(child_inode, 0 as usize, &mut data);
        if bytes_read.unwrap() < read_size {
            println!(
                "[kernel readtest] End of file reached, bytes read: {:?}",
                bytes_read
            );
        }
        let valid_data = &data[0..bytes_read.unwrap()];
        let text = String::from_utf8_lossy(&valid_data);
        let unescaped_data = unescape_char(&text);
        println!("[kernel readtest] Read Data at {:?}", path);
        print!("{}", unescaped_data);
    }
}

/// 将转义字符转换为实际的字符
fn unescape_char(escaped: &str) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < escaped.len() {
        // 确保没有越界，检查 i + 2 是否超出了 escaped 的长度
        if i + 2 <= escaped.len() {
            if &escaped[i..i + 2] == "\\n" {
                result.push('\n');
                i += 2;
            } else if &escaped[i..i + 2] == "\\r" {
                result.push('\r');
                i += 2;
            } else if &escaped[i..i + 2] == "\\t" {
                result.push('\t');
                i += 2;
            } else if &escaped[i..i + 2] == "\\\\" {
                result.push('\\');
                i += 2;
            } else if &escaped[i..i + 2] == "\\\"" {
                result.push('\"');
                i += 2;
            } else {
                // 如果没有匹配的转义字符，则正常添加字符
                result.push(escaped[i..i + 1].chars().next().unwrap());
                i += 1;
            }
        } else {
            // 如果无法匹配转义字符，直接添加当前字符
            result.push(escaped[i..i + 1].chars().next().unwrap());
            i += 1;
        }
    }

    result
}
