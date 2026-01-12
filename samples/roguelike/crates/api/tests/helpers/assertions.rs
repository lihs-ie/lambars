#![allow(dead_code)]

use serde_json::Value as JsonValue;

pub fn assert_json_has_key(json: &JsonValue, key: &str) {
    assert!(
        json.get(key).is_some(),
        "Expected JSON to have key '{}', but it was missing. JSON: {}",
        key,
        serde_json::to_string_pretty(json).unwrap_or_default()
    );
}

pub fn assert_json_string_eq(json: &JsonValue, key: &str, expected: &str) {
    let actual = json
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Expected key '{}' to be a string", key));

    assert_eq!(
        actual, expected,
        "Expected '{}' to be '{}', but got '{}'",
        key, expected, actual
    );
}

pub fn assert_json_u64_eq(json: &JsonValue, key: &str, expected: u64) {
    let actual = json
        .get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("Expected key '{}' to be a u64", key));

    assert_eq!(
        actual, expected,
        "Expected '{}' to be '{}', but got '{}'",
        key, expected, actual
    );
}

pub fn assert_json_array_len(json: &JsonValue, key: &str, expected_len: usize) {
    let array = json
        .get(key)
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("Expected key '{}' to be an array", key));

    assert_eq!(
        array.len(),
        expected_len,
        "Expected '{}' to have {} elements, but got {}",
        key,
        expected_len,
        array.len()
    );
}

pub fn assert_uuid_format(value: &str) {
    uuid::Uuid::parse_str(value)
        .unwrap_or_else(|_| panic!("Expected '{}' to be a valid UUID", value));
}

pub fn assert_error_response(json: &JsonValue, expected_code: &str) {
    assert_json_has_key(json, "error");
    let error = &json["error"];
    assert_json_has_key(error, "code");
    assert_json_has_key(error, "message");
    assert_json_string_eq(error, "code", expected_code);
}
