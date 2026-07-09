//! 日付ユーティリティ（純関数）。
//!
//! application 層は `SystemTime` / `UNIX_EPOCH` を直接使えない（architecture gate）。
//! そのため「UNIX 秒 → YYYY-MM-DD」への変換だけをここに集約し、delegation contract の
//! `expires` 比較（`expires >= today`）に使う。現在時刻の取得自体は infra clock 経由で行い、
//! ここには純粋な暦計算しか置かない。

/// UNIX 秒（UTC）を `YYYY-MM-DD` 文字列へ変換する。
///
/// `expires`（`YYYY-MM-DD`、ゼロ埋め固定長）との辞書順比較で日付大小を判定できる形を返す。
pub(crate) fn unix_seconds_to_ymd(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// `YYYY-MM-DD`（ゼロ埋め・実在日）かどうかを緩く検証する。
pub(crate) fn is_valid_ymd(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits_ok = [0, 1, 2, 3, 5, 6, 8, 9]
        .iter()
        .all(|&index| bytes[index].is_ascii_digit());
    if !digits_ok {
        return false;
    }
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..10].parse().unwrap_or(0);
    let year: i64 = value[0..4].parse().unwrap_or(0);
    if !(1..=12).contains(&month) || day < 1 {
        return false;
    }
    day <= days_in_month(year, month)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Howard Hinnant の civil-from-days アルゴリズム。
/// 1970-01-01 からの経過日数 `z` を (year, month, day) に変換する。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_known_epoch_dates() {
        assert_eq!(unix_seconds_to_ymd(0), "1970-01-01");
        // 2026-07-09 00:00:00 UTC = 1783555200
        assert_eq!(unix_seconds_to_ymd(1_783_555_200), "2026-07-09");
        // 2026-10-01 00:00:00 UTC = 1790812800
        assert_eq!(unix_seconds_to_ymd(1_790_812_800), "2026-10-01");
    }

    #[test]
    fn ymd_strings_order_lexicographically_by_date() {
        assert!("2026-07-09" < "2026-10-01");
        assert!("2026-09-30" < "2026-10-01");
        assert!("2026-10-01" <= "2026-10-01");
    }

    #[test]
    fn validates_ymd_format() {
        assert!(is_valid_ymd("2026-10-01"));
        assert!(is_valid_ymd("2024-02-29"));
        assert!(!is_valid_ymd("2026-13-01"));
        assert!(!is_valid_ymd("2026-02-30"));
        assert!(!is_valid_ymd("2023-02-29"));
        assert!(!is_valid_ymd("2026-1-1"));
        assert!(!is_valid_ymd("2026/10/01"));
        assert!(!is_valid_ymd("not-a-date"));
    }
}
