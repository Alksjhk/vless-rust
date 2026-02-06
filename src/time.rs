//! 时间工具模块
//!
//! 替代 chrono 库，提供时间戳、RFC3339 格式化和时间差计算功能

use std::time::{SystemTime, UNIX_EPOCH};

/// UTC 时间结构体
///
/// 替代 chrono::DateTime<Utc>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTime {
    timestamp: i64, // Unix 时间戳（秒）
}

impl UtcTime {
    /// 获取当前 UTC 时间
    pub fn now() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Self { timestamp }
    }

    /// 格式化为 RFC3339 字符串
    ///
    /// 示例: "2024-02-06T12:34:56Z"
    #[allow(clippy::wrong_self_convention)]
    pub fn to_rfc3339(&self) -> String {
        format_rfc3339(self.timestamp)
    }

    /// 计算时间差（秒）
    ///
    /// 返回 self - other 的秒数
    pub fn signed_duration_since(&self, other: UtcTime) -> i64 {
        self.timestamp - other.timestamp
    }
}

/// 快捷函数：获取当前 RFC3339 格式时间
pub fn utc_now_rfc3339() -> String {
    UtcTime::now().to_rfc3339()
}

/// 格式化 Unix 时间戳为 RFC3339 字符串
fn format_rfc3339(timestamp: i64) -> String {
    // 处理负时间戳（1970 年之前的日期）
    if timestamp < 0 {
        tracing::warn!("Negative timestamp {}, this may produce incorrect results", timestamp);
        // 继续处理，但结果可能不准确
    }

    // 手动实现 RFC3339 格式化
    // 使用标准算法计算日期时间

    const SECS_PER_MINUTE: i64 = 60;
    const SECS_PER_HOUR: i64 = 3600;
    const SECS_PER_DAY: i64 = 86400;
    const DAYS_PER_400_YEARS: i64 = 146097;
    const DAYS_PER_100_YEARS: i64 = 36524;
    const DAYS_PER_4_YEARS: i64 = 1461;
    const DAYS_PER_NORMAL_YEAR: i64 = 365;

    // 计算自 1970-01-01 以来的天数
    let days = timestamp / SECS_PER_DAY;
    let secs_of_day = timestamp % SECS_PER_DAY;

    // 计算 400 年周期
    let mut remaining_days = days;
    let cycles_400 = remaining_days / DAYS_PER_400_YEARS;
    remaining_days %= DAYS_PER_400_YEARS;

    // 计算 100 年周期
    let cycles_100 = remaining_days / DAYS_PER_100_YEARS;
    remaining_days %= DAYS_PER_100_YEARS;

    // 计算 4 年周期
    let cycles_4 = remaining_days / DAYS_PER_4_YEARS;
    remaining_days %= DAYS_PER_4_YEARS;

    // 计算剩余年份
    let years = remaining_days / DAYS_PER_NORMAL_YEAR;
    remaining_days %= DAYS_PER_NORMAL_YEAR;

    // 计算年份
    let year = 1970 + cycles_400 * 400 + cycles_100 * 100 + cycles_4 * 4 + years;

    // 计算月份和日期
    let mut month = 1;
    let mut day = remaining_days as i32 + 1;

    // 每月天数（非闰年）
    const MONTH_DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    // 判断是否为闰年
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);

    #[allow(clippy::needless_range_loop)]
    for m in 0..12 {
        let days_in_month = if m == 1 && is_leap {
            29 // 二月闰年
        } else {
            MONTH_DAYS[m]
        };

        if day <= days_in_month {
            month = m + 1;
            break;
        }
        day -= days_in_month;
    }

    // 计算时分秒（处理负数的秒数）
    let secs_of_day = if secs_of_day < 0 { secs_of_day + SECS_PER_DAY } else { secs_of_day };
    let hour = (secs_of_day / SECS_PER_HOUR) as i32;
    let minute = ((secs_of_day % SECS_PER_HOUR) / SECS_PER_MINUTE) as i32;
    let second = (secs_of_day % SECS_PER_MINUTE) as i32;

    // 格式化为 RFC3339
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}
