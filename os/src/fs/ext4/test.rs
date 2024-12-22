use alloc::{
    format,
    string::{String, ToString},
    vec,
};

use super::ext4fs::Ext4FileSystem;

impl Ext4FileSystem {
    /// 尝试打开一个文件并读取内容
    /// 读取 2048 个字节
    pub fn test_get_file(&self, path: &str) {
        // 读取大小 1G
        let read_size = 2048;
        let mut read_buf = vec![0u8; read_size as usize];
        let child_inode = self.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
        let mut data = vec![0u8; read_size as usize];
        // 读取文件内容
        let read_data = self.read_at(child_inode, 0 as usize, &mut data);
        let text = String::from_utf8_lossy(&data);
        let unescaped_data = unescape_char(&text);

        println!("Read data (escaped): {}", unescaped_data);
    }
}

// 转义字符
fn escape_char(byte: u8) -> String {
    match byte {
        0x20..=0x7E => (byte as char).to_string(), // 可打印字符直接输出
        0x09 => "\\t".to_string(),                 // 制表符
        0x0A => "\\n".to_string(),                 // 换行符
        0x0D => "\\r".to_string(),                 // 回车符
        0x22 => "\\\"".to_string(),                // 双引号
        0x5C => "\\\\".to_string(),                // 反斜杠
        _ => format!("\\x{:02X}", byte),           // 非打印字符以十六进制显示
    }
}

/// 将转义字符转换为实际的字符
fn unescape_char(escaped: &str) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < escaped.len() {
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
            result.push(escaped[i..i + 1].chars().next().unwrap());
            i += 1;
        }
    }

    result
}
