use chrono::{DateTime, Duration, NaiveDate, Utc, Datelike};

use crate::error::TodoError;

/// Parses due date input. Accepts:
/// - Absolute: "YYYY-MM-DD" -> end of that day (23:59:59 UTC)
/// - Relative days: "3d" -> now + 3 days
/// - Relative weeks: "2w" -> now + 2 weeks (14 days)
pub fn parse_due(input: &str) -> Result<DateTime<Utc>, TodoError> {
    let input = input.trim();

    // Try relative days: e.g. "3d"
    if let Some(num_str) = input.strip_suffix('d') {
        let days: i64 = num_str
            .parse()
            .map_err(|_| TodoError::ParseError(format!("Invalid relative days: {}", input)))?;
        return Ok(Utc::now() + Duration::days(days));
    }

    // Try relative weeks: e.g. "2w"
    if let Some(num_str) = input.strip_suffix('w') {
        let weeks: i64 = num_str
            .parse()
            .map_err(|_| TodoError::ParseError(format!("Invalid relative weeks: {}", input)))?;
        return Ok(Utc::now() + Duration::weeks(weeks));
    }

    // Try absolute date: "YYYY-MM-DD"
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let end_of_day = date
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| TodoError::ParseError(format!("Invalid date: {}", input)))?;
        return Ok(end_of_day.and_utc());
    }

    Err(TodoError::ParseError(format!(
        "Unrecognized due date format: '{}'. Use YYYY-MM-DD, Nd, or Nw.",
        input
    )))
}

/// Parses --since filter input. Accepts:
/// - "today" -> start of today (00:00:00 UTC)
/// - Relative days: "7d" -> now - 7 days
/// - Absolute: "YYYY-MM-DD" -> start of that day (00:00:00 UTC)
pub fn parse_since(input: &str) -> Result<DateTime<Utc>, TodoError> {
    let input = input.trim();

    if input.eq_ignore_ascii_case("today") {
        let now = Utc::now();
        let start_of_today = NaiveDate::from_ymd_opt(now.year(), now.month(), now.day())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .ok_or_else(|| TodoError::ParseError("Failed to compute start of today".into()))?;
        return Ok(start_of_today.and_utc());
    }

    // Try relative days: e.g. "7d" -> now - 7 days
    if let Some(num_str) = input.strip_suffix('d') {
        let days: i64 = num_str
            .parse()
            .map_err(|_| TodoError::ParseError(format!("Invalid relative days: {}", input)))?;
        return Ok(Utc::now() - Duration::days(days));
    }

    // Try absolute date: "YYYY-MM-DD"
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let start_of_day = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| TodoError::ParseError(format!("Invalid date: {}", input)))?;
        return Ok(start_of_day.and_utc());
    }

    Err(TodoError::ParseError(format!(
        "Unrecognized since format: '{}'. Use 'today', Nd, or YYYY-MM-DD.",
        input
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn parse_absolute_date() {
        let result = parse_due("2026-03-15").unwrap();
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 3);
        assert_eq!(result.day(), 15);
        // Should be end of day
        assert_eq!(result.hour(), 23);
        assert_eq!(result.minute(), 59);
        assert_eq!(result.second(), 59);
    }

    #[test]
    fn parse_relative_days() {
        let before = Utc::now() + Duration::days(3);
        let result = parse_due("3d").unwrap();
        let after = Utc::now() + Duration::days(3);
        // Within 2 seconds tolerance
        assert!((result - before).num_seconds().abs() <= 2);
        assert!((result - after).num_seconds().abs() <= 2);
    }

    #[test]
    fn parse_relative_weeks() {
        let before = Utc::now() + Duration::weeks(2);
        let result = parse_due("2w").unwrap();
        let after = Utc::now() + Duration::weeks(2);
        assert!((result - before).num_seconds().abs() <= 2);
        assert!((result - after).num_seconds().abs() <= 2);
    }

    #[test]
    fn parse_today() {
        let result = parse_since("today").unwrap();
        let now = Utc::now();
        assert_eq!(result.year(), now.year());
        assert_eq!(result.month(), now.month());
        assert_eq!(result.day(), now.day());
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(parse_due("xyz").is_err());
        assert!(parse_since("xyz").is_err());
    }
}
