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

    #[test]
    fn test_format_last_played() {
        assert_eq!(format_last_played(""), "Never played");
        assert_eq!(format_last_played("2023-01-01T12:00:00Z"), "Last: 2023-01-01");
        assert_eq!(format_last_played("invalid"), "Last: invalid"); // Implementation detail: splits on T
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30m");
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(90), "1h 30m");
        assert_eq!(format_duration(125), "2h 5m");
    }

    #[test]
    fn test_format_time_mm_ss() {
        assert_eq!(format_time_mm_ss(0.0), "0:00");
        assert_eq!(format_time_mm_ss(65.5), "1:05");
        assert_eq!(format_time_mm_ss(3599.0), "59:59");
    }
}
