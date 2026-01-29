//! Timestamp value object.
//!
//! Provides a strongly-typed UTC timestamp for events and records.

use std::cmp::Ordering;
use std::fmt;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// A UTC timestamp for recording when events occurred.
///
/// `Timestamp` wraps `chrono::DateTime<Utc>` to provide:
///
/// - **Type safety**: Explicit type for timestamps vs other date/time values
/// - **UTC guarantee**: All timestamps are in UTC timezone
/// - **Ordering**: Implements `Ord` for sorting and comparison
///
/// # Examples
///
/// ```rust
/// use bank::domain::value_objects::Timestamp;
///
/// // Get the current timestamp
/// let now = Timestamp::now();
///
/// // Parse from an ISO 8601 string
/// let parsed = Timestamp::parse("2024-01-15T10:30:00Z");
/// assert!(parsed.is_some());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a timestamp representing the current moment in UTC.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let now = Timestamp::now();
    /// // Timestamp is in UTC
    /// ```
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Creates a timestamp from Unix epoch seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Seconds since Unix epoch (1970-01-01 00:00:00 UTC)
    ///
    /// # Returns
    ///
    /// * `Some(Timestamp)` if the seconds represent a valid timestamp
    /// * `None` if the seconds are out of range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::from_unix_seconds(1_705_312_200);
    /// assert!(ts.is_some());
    /// ```
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        Utc.timestamp_opt(seconds, 0).single().map(Self)
    }

    /// Creates a timestamp from Unix epoch milliseconds.
    ///
    /// # Arguments
    ///
    /// * `milliseconds` - Milliseconds since Unix epoch
    ///
    /// # Returns
    ///
    /// * `Some(Timestamp)` if the milliseconds represent a valid timestamp
    /// * `None` if the milliseconds are out of range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::from_unix_millis(1_705_312_200_000);
    /// assert!(ts.is_some());
    /// ```
    #[must_use]
    pub fn from_unix_millis(milliseconds: i64) -> Option<Self> {
        Utc.timestamp_millis_opt(milliseconds).single().map(Self)
    }

    /// Parses a timestamp from an ISO 8601 formatted string.
    ///
    /// Supported formats:
    /// - `2024-01-15T10:30:00Z`
    /// - `2024-01-15T10:30:00+00:00`
    /// - `2024-01-15T10:30:00.123Z`
    ///
    /// # Arguments
    ///
    /// * `value` - An ISO 8601 formatted date-time string
    ///
    /// # Returns
    ///
    /// * `Some(Timestamp)` if parsing succeeds
    /// * `None` if parsing fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::parse("2024-01-15T10:30:00Z");
    /// assert!(ts.is_some());
    ///
    /// let invalid = Timestamp::parse("not-a-timestamp");
    /// assert!(invalid.is_none());
    /// ```
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        // Try parsing as DateTime<Utc> first
        if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
            return Some(Self(datetime.with_timezone(&Utc)));
        }

        // Try parsing without timezone (assume UTC)
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
            return Some(Self(Utc.from_utc_datetime(&naive)));
        }

        // Try parsing with milliseconds
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(Self(Utc.from_utc_datetime(&naive)));
        }

        None
    }

    /// Creates a timestamp from a `DateTime<Utc>`.
    ///
    /// # Arguments
    ///
    /// * `datetime` - A `chrono::DateTime<Utc>` value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    /// use chrono::Utc;
    ///
    /// let datetime = Utc::now();
    /// let ts = Timestamp::from_datetime(datetime);
    /// ```
    #[must_use]
    pub const fn from_datetime(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }

    /// Returns the underlying `DateTime<Utc>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::now();
    /// let datetime = ts.as_datetime();
    /// ```
    #[must_use]
    pub const fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Returns the timestamp as Unix epoch seconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::now();
    /// let seconds = ts.unix_seconds();
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // chrono::DateTime::timestamp is not const
    pub fn unix_seconds(&self) -> i64 {
        self.0.timestamp()
    }

    /// Returns the timestamp as Unix epoch milliseconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::now();
    /// let millis = ts.unix_millis();
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // chrono::DateTime::timestamp_millis is not const
    pub fn unix_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    /// Returns the ISO 8601 formatted string representation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts = Timestamp::parse("2024-01-15T10:30:00Z").unwrap();
    /// assert!(ts.to_iso_string().starts_with("2024-01-15"));
    /// ```
    #[must_use]
    pub fn to_iso_string(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Returns the duration between this timestamp and another.
    ///
    /// A positive duration means `other` is after `self`.
    /// A negative duration means `other` is before `self`.
    ///
    /// # Arguments
    ///
    /// * `other` - Another timestamp to compare against
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
    /// let ts2 = Timestamp::from_unix_seconds(1100).unwrap();
    ///
    /// let duration = ts1.duration_until(&ts2);
    /// assert_eq!(duration.num_seconds(), 100);
    /// ```
    #[must_use]
    pub fn duration_until(&self, other: &Self) -> chrono::Duration {
        other.0.signed_duration_since(self.0)
    }

    /// Returns `true` if this timestamp is before another.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
    /// let ts2 = Timestamp::from_unix_seconds(2000).unwrap();
    ///
    /// assert!(ts1.is_before(&ts2));
    /// assert!(!ts2.is_before(&ts1));
    /// ```
    #[must_use]
    pub fn is_before(&self, other: &Self) -> bool {
        self.0 < other.0
    }

    /// Returns `true` if this timestamp is after another.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::Timestamp;
    ///
    /// let ts1 = Timestamp::from_unix_seconds(2000).unwrap();
    /// let ts2 = Timestamp::from_unix_seconds(1000).unwrap();
    ///
    /// assert!(ts1.is_after(&ts2));
    /// assert!(!ts2.is_after(&ts1));
    /// ```
    #[must_use]
    pub fn is_after(&self, other: &Self) -> bool {
        self.0 > other.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.to_iso_string())
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Timestamp::now Tests
    // =========================================================================

    #[rstest]
    fn now_returns_current_time() {
        let before = Utc::now();
        let timestamp = Timestamp::now();
        let after = Utc::now();

        assert!(timestamp.as_datetime() >= &before);
        assert!(timestamp.as_datetime() <= &after);
    }

    #[rstest]
    fn now_produces_different_values_over_time() {
        let ts1 = Timestamp::now();
        // Small operation to ensure time passes
        let _ = (0..1000).sum::<i32>();
        let ts2 = Timestamp::now();

        // They should be equal or ts2 should be later
        assert!(ts1 <= ts2);
    }

    // =========================================================================
    // Timestamp::from_unix_seconds Tests
    // =========================================================================

    #[rstest]
    fn from_unix_seconds_valid() {
        let result = Timestamp::from_unix_seconds(1_705_312_200);

        assert!(result.is_some());
        let ts = result.unwrap();
        assert_eq!(ts.unix_seconds(), 1_705_312_200);
    }

    #[rstest]
    fn from_unix_seconds_epoch() {
        let result = Timestamp::from_unix_seconds(0);

        assert!(result.is_some());
        let ts = result.unwrap();
        assert_eq!(ts.unix_seconds(), 0);
    }

    #[rstest]
    fn from_unix_seconds_negative() {
        let result = Timestamp::from_unix_seconds(-86400); // One day before epoch

        assert!(result.is_some());
        let ts = result.unwrap();
        assert_eq!(ts.unix_seconds(), -86400);
    }

    // =========================================================================
    // Timestamp::from_unix_millis Tests
    // =========================================================================

    #[rstest]
    fn from_unix_millis_valid() {
        let result = Timestamp::from_unix_millis(1_705_312_200_000);

        assert!(result.is_some());
        let ts = result.unwrap();
        assert_eq!(ts.unix_millis(), 1_705_312_200_000);
    }

    #[rstest]
    fn from_unix_millis_with_precision() {
        let result = Timestamp::from_unix_millis(1_705_312_200_123);

        assert!(result.is_some());
        let ts = result.unwrap();
        assert_eq!(ts.unix_millis(), 1_705_312_200_123);
    }

    // =========================================================================
    // Timestamp::parse Tests
    // =========================================================================

    #[rstest]
    fn parse_iso8601_with_z() {
        let result = Timestamp::parse("2024-01-15T10:30:00Z");

        assert!(result.is_some());
    }

    #[rstest]
    fn parse_iso8601_with_offset() {
        let result = Timestamp::parse("2024-01-15T10:30:00+00:00");

        assert!(result.is_some());
    }

    #[rstest]
    fn parse_iso8601_with_millis() {
        let result = Timestamp::parse("2024-01-15T10:30:00.123Z");

        assert!(result.is_some());
    }

    #[rstest]
    fn parse_invalid_string_returns_none() {
        let result = Timestamp::parse("not-a-timestamp");

        assert!(result.is_none());
    }

    #[rstest]
    fn parse_empty_string_returns_none() {
        let result = Timestamp::parse("");

        assert!(result.is_none());
    }

    #[rstest]
    fn parse_partial_date_returns_none() {
        let result = Timestamp::parse("2024-01-15");

        assert!(result.is_none());
    }

    // =========================================================================
    // Timestamp::from_datetime Tests
    // =========================================================================

    #[rstest]
    fn from_datetime_creates_timestamp() {
        let datetime = Utc::now();
        let timestamp = Timestamp::from_datetime(datetime);

        assert_eq!(*timestamp.as_datetime(), datetime);
    }

    // =========================================================================
    // Timestamp::unix_seconds Tests
    // =========================================================================

    #[rstest]
    fn unix_seconds_returns_correct_value() {
        let ts = Timestamp::from_unix_seconds(1_705_312_200).unwrap();

        assert_eq!(ts.unix_seconds(), 1_705_312_200);
    }

    // =========================================================================
    // Timestamp::unix_millis Tests
    // =========================================================================

    #[rstest]
    fn unix_millis_returns_correct_value() {
        let ts = Timestamp::from_unix_millis(1_705_312_200_123).unwrap();

        assert_eq!(ts.unix_millis(), 1_705_312_200_123);
    }

    // =========================================================================
    // Timestamp::to_iso_string Tests
    // =========================================================================

    #[rstest]
    fn to_iso_string_formats_correctly() {
        let ts = Timestamp::parse("2024-01-15T10:30:00Z").unwrap();
        let iso_string = ts.to_iso_string();

        assert!(iso_string.starts_with("2024-01-15"));
        assert!(iso_string.contains("10:30:00"));
    }

    // =========================================================================
    // Timestamp::duration_until Tests
    // =========================================================================

    #[rstest]
    fn duration_until_positive() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1100).unwrap();

        let duration = ts1.duration_until(&ts2);

        assert_eq!(duration.num_seconds(), 100);
    }

    #[rstest]
    fn duration_until_negative() {
        let ts1 = Timestamp::from_unix_seconds(1100).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        let duration = ts1.duration_until(&ts2);

        assert_eq!(duration.num_seconds(), -100);
    }

    #[rstest]
    fn duration_until_zero() {
        let ts = Timestamp::from_unix_seconds(1000).unwrap();

        let duration = ts.duration_until(&ts);

        assert_eq!(duration.num_seconds(), 0);
    }

    // =========================================================================
    // Timestamp::is_before Tests
    // =========================================================================

    #[rstest]
    fn is_before_returns_true_when_earlier() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(2000).unwrap();

        assert!(ts1.is_before(&ts2));
    }

    #[rstest]
    fn is_before_returns_false_when_later() {
        let ts1 = Timestamp::from_unix_seconds(2000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert!(!ts1.is_before(&ts2));
    }

    #[rstest]
    fn is_before_returns_false_when_equal() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert!(!ts1.is_before(&ts2));
    }

    // =========================================================================
    // Timestamp::is_after Tests
    // =========================================================================

    #[rstest]
    fn is_after_returns_true_when_later() {
        let ts1 = Timestamp::from_unix_seconds(2000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert!(ts1.is_after(&ts2));
    }

    #[rstest]
    fn is_after_returns_false_when_earlier() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(2000).unwrap();

        assert!(!ts1.is_after(&ts2));
    }

    #[rstest]
    fn is_after_returns_false_when_equal() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert!(!ts1.is_after(&ts2));
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_formats_as_iso8601() {
        let ts = Timestamp::parse("2024-01-15T10:30:00Z").unwrap();
        let display = format!("{ts}");

        assert!(display.contains("2024-01-15"));
    }

    // =========================================================================
    // Ord Tests
    // =========================================================================

    #[rstest]
    fn ord_earlier_is_less() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(2000).unwrap();

        assert_eq!(ts1.cmp(&ts2), Ordering::Less);
    }

    #[rstest]
    fn ord_later_is_greater() {
        let ts1 = Timestamp::from_unix_seconds(2000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert_eq!(ts1.cmp(&ts2), Ordering::Greater);
    }

    #[rstest]
    fn ord_equal_is_equal() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        assert_eq!(ts1.cmp(&ts2), Ordering::Equal);
    }

    #[rstest]
    fn partial_ord_consistent_with_ord() {
        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(2000).unwrap();

        assert_eq!(ts1.partial_cmp(&ts2), Some(Ordering::Less));
    }

    // =========================================================================
    // From/Into Tests
    // =========================================================================

    #[rstest]
    fn from_datetime_utc() {
        let datetime = Utc::now();
        let timestamp: Timestamp = datetime.into();

        assert_eq!(*timestamp.as_datetime(), datetime);
    }

    #[rstest]
    fn into_datetime_utc() {
        let timestamp = Timestamp::now();
        let expected = *timestamp.as_datetime();
        let datetime: DateTime<Utc> = timestamp.into();

        assert_eq!(datetime, expected);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_roundtrip() {
        let original = Timestamp::now();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Timestamp = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // Hash Tests
    // =========================================================================

    #[rstest]
    fn hash_is_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let ts = Timestamp::from_unix_seconds(1000).unwrap();

        let mut hasher1 = DefaultHasher::new();
        ts.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        ts.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[rstest]
    fn equal_timestamps_have_same_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let ts1 = Timestamp::from_unix_seconds(1000).unwrap();
        let ts2 = Timestamp::from_unix_seconds(1000).unwrap();

        let mut hasher1 = DefaultHasher::new();
        ts1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        ts2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }
}
