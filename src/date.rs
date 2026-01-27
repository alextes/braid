//! date parsing utilities for scheduled_for field.

use time::{Duration, OffsetDateTime, Time};

use crate::error::{BrdError, Result};

/// parse a date string into an OffsetDateTime.
/// supports:
/// - ISO dates: "2025-02-15" (midnight UTC)
/// - relative days: "+7d" (7 days from now)
/// - relative weeks: "+2w" (2 weeks from now)
/// - relative months: "+1mo" (1 month from now, approximated as 30 days)
/// - "tomorrow" (next day, midnight UTC)
pub fn parse_scheduled_date(input: &str) -> Result<OffsetDateTime> {
    let input = input.trim().to_lowercase();

    if input == "tomorrow" {
        let now = OffsetDateTime::now_utc();
        let tomorrow = now + Duration::days(1);
        return Ok(tomorrow.replace_time(Time::MIDNIGHT));
    }

    // relative formats: +Nd, +Nw, +Nmo
    if let Some(rest) = input.strip_prefix('+') {
        if let Some(days_str) = rest.strip_suffix('d') {
            let days: i64 = days_str.parse().map_err(|_| {
                BrdError::ParseError("date".into(), format!("invalid days: {}", rest))
            })?;
            let target = OffsetDateTime::now_utc() + Duration::days(days);
            return Ok(target);
        }

        if let Some(weeks_str) = rest.strip_suffix('w') {
            let weeks: i64 = weeks_str.parse().map_err(|_| {
                BrdError::ParseError("date".into(), format!("invalid weeks: {}", rest))
            })?;
            let target = OffsetDateTime::now_utc() + Duration::weeks(weeks);
            return Ok(target);
        }

        if let Some(months_str) = rest.strip_suffix("mo") {
            let months: i64 = months_str.parse().map_err(|_| {
                BrdError::ParseError("date".into(), format!("invalid months: {}", rest))
            })?;
            // approximate 1 month as 30 days
            let target = OffsetDateTime::now_utc() + Duration::days(months * 30);
            return Ok(target);
        }

        return Err(BrdError::ParseError(
            "date".into(),
            format!(
                "invalid relative format '{}'. use +Nd, +Nw, or +Nmo (e.g., +7d, +2w, +1mo)",
                input
            ),
        ));
    }

    // ISO date format: YYYY-MM-DD
    if input.len() == 10 && input.chars().nth(4) == Some('-') && input.chars().nth(7) == Some('-') {
        let format = time::format_description::parse("[year]-[month]-[day]")
            .map_err(|e| BrdError::ParseError("date".into(), format!("format error: {}", e)))?;
        let date = time::Date::parse(&input, &format).map_err(|e| {
            BrdError::ParseError("date".into(), format!("invalid date '{}': {}", input, e))
        })?;
        return Ok(date.with_time(Time::MIDNIGHT).assume_utc());
    }

    Err(BrdError::ParseError(
        "date".into(),
        format!(
            "invalid date format '{}'. use YYYY-MM-DD, +Nd, +Nw, +Nmo, or 'tomorrow'",
            input
        ),
    ))
}

/// format a future scheduled date for display.
/// returns "in Xh", "in Xd", "in Xw", or "in Xmo" format.
pub fn format_scheduled(scheduled_for: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = scheduled_for - now;
    let minutes = duration.whole_minutes();

    if minutes <= 0 {
        return "now".to_string();
    }

    let hours = duration.whole_hours();
    if hours < 24 {
        return format!("in {}h", hours.max(1));
    }

    let days = duration.whole_days();
    if days < 7 {
        return format!("in {}d", days);
    }

    if days < 30 {
        return format!("in {}w", days / 7);
    }

    let months = days / 30;
    format!("in {}mo", months)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_parse_tomorrow() {
        let result = parse_scheduled_date("tomorrow").unwrap();
        let now = OffsetDateTime::now_utc();
        let expected_date = (now + Duration::days(1)).date();
        assert_eq!(result.date(), expected_date);
        assert_eq!(result.time(), Time::MIDNIGHT);
    }

    #[test]
    fn test_parse_relative_days() {
        let result = parse_scheduled_date("+7d").unwrap();
        let now = OffsetDateTime::now_utc();
        let diff = result - now;
        // allow some tolerance for test execution time
        assert!(diff.whole_days() >= 6 && diff.whole_days() <= 7);
    }

    #[test]
    fn test_parse_relative_weeks() {
        let result = parse_scheduled_date("+2w").unwrap();
        let now = OffsetDateTime::now_utc();
        let diff = result - now;
        assert!(diff.whole_days() >= 13 && diff.whole_days() <= 14);
    }

    #[test]
    fn test_parse_relative_months() {
        let result = parse_scheduled_date("+1mo").unwrap();
        let now = OffsetDateTime::now_utc();
        let diff = result - now;
        assert!(diff.whole_days() >= 29 && diff.whole_days() <= 30);
    }

    #[test]
    fn test_parse_iso_date() {
        let result = parse_scheduled_date("2025-12-25").unwrap();
        assert_eq!(result.year(), 2025);
        assert_eq!(result.month() as u8, 12);
        assert_eq!(result.day(), 25);
        assert_eq!(result.time(), Time::MIDNIGHT);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_scheduled_date("invalid").is_err());
        assert!(parse_scheduled_date("+abc").is_err());
        assert!(parse_scheduled_date("2025-13-01").is_err());
    }

    #[test]
    fn test_format_scheduled_hours() {
        let now = OffsetDateTime::now_utc();
        // add extra time to avoid boundary issues
        let result = format_scheduled(now + Duration::hours(5) + Duration::minutes(30));
        assert!(result.starts_with("in ") && result.ends_with("h"));
    }

    #[test]
    fn test_format_scheduled_days() {
        let now = OffsetDateTime::now_utc();
        // add 2 days to be safely in the "days" range
        let result = format_scheduled(now + Duration::days(2));
        assert!(result.starts_with("in ") && result.ends_with("d"));
    }

    #[test]
    fn test_format_scheduled_weeks() {
        let now = OffsetDateTime::now_utc();
        // add 10 days to be safely in the "weeks" range
        let result = format_scheduled(now + Duration::days(10));
        assert!(result.starts_with("in ") && result.ends_with("w"));
    }

    #[test]
    fn test_format_scheduled_months() {
        let now = OffsetDateTime::now_utc();
        // add 45 days to be safely in the "months" range
        let result = format_scheduled(now + Duration::days(45));
        assert!(result.starts_with("in ") && result.ends_with("mo"));
    }

    #[test]
    fn test_format_scheduled_past() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(format_scheduled(now - Duration::days(1)), "now");
    }
}
