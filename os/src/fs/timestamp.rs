use alloc::{format, string::String};

pub const SECONDS_PER_DAY: u32 = 86400;
pub const DAYS_PER_LITTLE_MONTH: u8 = 30;
pub const DAYS_PER_BIG_MONTH: u8 = 31;
pub const DAYS_PER_FEBRUARY: u8 = 28;

// 闰年判断
pub fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && year % 100 != 0 || year % 400 == 0
}

/// 时间格式化
/// # 参数
/// + timestamp: 时间戳 单位为秒
/// # 返回值
/// + 计算好的标准格式的时间 xxxx-xx-xx xx:xx:xx
/// # 说明
/// + 返回的时间是UTC时间，所以与本地时间也就是北京时间相差8小时
pub fn format_time(timestamp: u32) -> String {
    let mut days_since_epoch = timestamp / SECONDS_PER_DAY;
    let remaining_seconds = timestamp % SECONDS_PER_DAY;

    // 计算时间
    let hours = remaining_seconds / 3600;
    let minutes = (remaining_seconds % 3600) / 60;
    let seconds = remaining_seconds % 60;

    // 计算年份
    let mut year = 1970;
    while days_since_epoch >= if is_leap_year(year) { 366 } else { 365 } {
        days_since_epoch -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    // 计算月份
    let mut month = 1;
    while days_since_epoch >= days_of_month(year, month) as u32 {
        days_since_epoch -= days_of_month(year, month) as u32;
        month += 1;
    }

    // 天数转换为当前月的日期
    let day = days_since_epoch + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

// 月份天数
pub fn days_of_month(year: u32, month: u8) -> u8 {
    match month {
        1 => DAYS_PER_BIG_MONTH,
        2 => {
            if is_leap_year(year) {
                DAYS_PER_FEBRUARY + 1
            } else {
                DAYS_PER_FEBRUARY
            }
        }
        3 => DAYS_PER_BIG_MONTH,
        4 => DAYS_PER_LITTLE_MONTH,
        5 => DAYS_PER_BIG_MONTH,
        6 => DAYS_PER_LITTLE_MONTH,
        7 => DAYS_PER_BIG_MONTH,
        8 => DAYS_PER_BIG_MONTH,
        9 => DAYS_PER_LITTLE_MONTH,
        10 => DAYS_PER_BIG_MONTH,
        11 => DAYS_PER_LITTLE_MONTH,
        12 => DAYS_PER_BIG_MONTH,
        _ => 0,
    }
}
