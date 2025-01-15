pub fn path_check(path: &str, is_goal: &mut bool) -> usize {
    // 遍历字符串中的每个字符及其索引
    for (i, c) in path.chars().enumerate() {
        // 检查是否到达文件名的最大长度限制
        if i >= 255 {
            break;
        }

        // 检查字符是否是路径分隔符
        if c == '/' {
            *is_goal = false;
            return i;
        }

        // 检查是否达到字符串结尾
        if c == '\0' {
            *is_goal = true;
            return i;
        }
    }

    // 如果没有找到 '/' 或 '\0'，且长度小于最大文件名长度
    *is_goal = true;
    path.len()
}
