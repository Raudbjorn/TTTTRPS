//! Formatting utilities for display

/// Format a timestamp to a human-readable "last played" string
pub fn format_last_played(timestamp: &str) -> String {
    if timestamp.is_empty() {
        return "Never played".to_string();
    }
    if let Some(date_part) = timestamp.split('T').next() {
        return format!("Last: {}", date_part);
    }
    "Recently".to_string()
}

/// Format duration in minutes to human-readable string
pub fn format_duration(minutes: i64) -> String {
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    }
}

/// Format seconds to MM:SS
pub fn format_time_mm_ss(seconds: f32) -> String {
    let mins = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{}:{:02}", mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // format_last_played Tests
    // ========================================================================

    #[test]
    fn test_format_last_played_empty() {
        assert_eq!(format_last_played(""), "Never played");
    }

    #[test]
    fn test_format_last_played_iso_timestamp() {
        assert_eq!(format_last_played("2023-01-01T12:00:00Z"), "Last: 2023-01-01");
        assert_eq!(format_last_played("2024-12-31T23:59:59.999Z"), "Last: 2024-12-31");
    }

    #[test]
    fn test_format_last_played_with_timezone_offset() {
        assert_eq!(format_last_played("2023-06-15T10:30:00+02:00"), "Last: 2023-06-15");
    }

    #[test]
    fn test_format_last_played_date_only() {
        // No T separator means entire string is returned
        assert_eq!(format_last_played("2023-01-01"), "Last: 2023-01-01");
    }

    #[test]
    fn test_format_last_played_invalid_format() {
        assert_eq!(format_last_played("invalid"), "Last: invalid");
        assert_eq!(format_last_played("not-a-date"), "Last: not-a-date");
    }

    #[test]
    fn test_format_last_played_whitespace_only() {
        // Whitespace is not empty, so it's treated as a date string
        assert_eq!(format_last_played("   "), "Last:    ");
    }

    // ========================================================================
    // format_duration Tests
    // ========================================================================

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0m");
    }

    #[test]
    fn test_format_duration_under_hour() {
        assert_eq!(format_duration(1), "1m");
        assert_eq!(format_duration(30), "30m");
        assert_eq!(format_duration(59), "59m");
    }

    #[test]
    fn test_format_duration_exact_hours() {
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(120), "2h");
        assert_eq!(format_duration(180), "3h");
    }

    #[test]
    fn test_format_duration_hours_and_minutes() {
        assert_eq!(format_duration(61), "1h 1m");
        assert_eq!(format_duration(90), "1h 30m");
        assert_eq!(format_duration(125), "2h 5m");
        assert_eq!(format_duration(179), "2h 59m");
    }

    #[test]
    fn test_format_duration_large_values() {
        assert_eq!(format_duration(600), "10h");
        assert_eq!(format_duration(1440), "24h"); // 1 day
        assert_eq!(format_duration(10080), "168h"); // 1 week
    }

    #[test]
    fn test_format_duration_negative() {
        // Negative values are handled by Rust's modulo behavior
        // -30 / 60 = 0, -30 % 60 = -30
        assert_eq!(format_duration(-30), "-30m");
    }

    // ========================================================================
    // format_time_mm_ss Tests
    // ========================================================================

    #[test]
    fn test_format_time_zero() {
        assert_eq!(format_time_mm_ss(0.0), "0:00");
    }

    #[test]
    fn test_format_time_under_minute() {
        assert_eq!(format_time_mm_ss(1.0), "0:01");
        assert_eq!(format_time_mm_ss(30.0), "0:30");
        assert_eq!(format_time_mm_ss(59.9), "0:59");
    }

    #[test]
    fn test_format_time_exact_minutes() {
        assert_eq!(format_time_mm_ss(60.0), "1:00");
        assert_eq!(format_time_mm_ss(120.0), "2:00");
        assert_eq!(format_time_mm_ss(300.0), "5:00");
    }

    #[test]
    fn test_format_time_minutes_and_seconds() {
        assert_eq!(format_time_mm_ss(65.5), "1:05");
        assert_eq!(format_time_mm_ss(90.0), "1:30");
        assert_eq!(format_time_mm_ss(125.0), "2:05");
    }

    #[test]
    fn test_format_time_large_values() {
        assert_eq!(format_time_mm_ss(3599.0), "59:59");
        assert_eq!(format_time_mm_ss(3600.0), "60:00"); // 1 hour
        assert_eq!(format_time_mm_ss(7200.0), "120:00"); // 2 hours
    }

    #[test]
    fn test_format_time_fractional_seconds() {
        // Fractional seconds are floored
        assert_eq!(format_time_mm_ss(0.1), "0:00");
        assert_eq!(format_time_mm_ss(0.9), "0:00");
        assert_eq!(format_time_mm_ss(1.9), "0:01");
        assert_eq!(format_time_mm_ss(59.999), "0:59");
    }

    #[test]
    fn test_format_time_seconds_padding() {
        // Seconds should always be zero-padded to 2 digits
        assert_eq!(format_time_mm_ss(1.0), "0:01");
        assert_eq!(format_time_mm_ss(9.0), "0:09");
        assert_eq!(format_time_mm_ss(10.0), "0:10");
    }
}
