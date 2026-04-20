use super::*;

#[test]
fn csv_escape_simple() { assert_eq!(csv_escape("hello"), "hello"); }

#[test]
fn csv_escape_comma() { assert_eq!(csv_escape("a,b"), "\"a,b\""); }

#[test]
fn csv_escape_quote() { assert_eq!(csv_escape("a\"b"), "\"a\"\"b\""); }

#[test]
fn csv_escape_newline() { assert_eq!(csv_escape("a\nb"), "\"a\nb\""); }

#[test]
fn to_csv_basic() {
    let rows = vec![("1".to_string(), "hello".to_string()), ("2".to_string(), "a,b".to_string())];
    let csv = to_csv(&rows, &["id", "val"], |r| vec![r.0.clone(), r.1.clone()]);
    assert_eq!(csv, "id,val\n1,hello\n2,\"a,b\"\n");
}

#[test]
fn to_csv_empty() {
    let rows: Vec<(String,)> = vec![];
    let csv = to_csv(&rows, &["id"], |r| vec![r.0.clone()]);
    assert_eq!(csv, "id\n");
}

#[test]
fn serialize_rows_json() {
    let rows = vec![serde_json::json!({"a": 1}), serde_json::json!({"a": 2})];
    let result = serialize_rows(&rows, "json", |_| vec![], &[]).unwrap();
    assert_eq!(result.row_count, 2);
    assert!(result.data.contains("\"a\":1") || result.data.contains("\"a\": 1"));
}

#[test]
fn serialize_rows_unknown_format() {
    let rows: Vec<String> = vec![];
    let result = serialize_rows(&rows, "xml", |_| vec![], &[]);
    assert!(result.is_err());
}
