//! Canonical byte/JSON ingress for .sdc and standby project images.
//! Parsing owns no AppState, publication, media or output side effects.
use crate::PROJECT_FILE_MAX_BYTES;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::{fmt, fs::File, io::Read, path::Path};

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueValueVisitor)
    }
}

struct UniqueValueVisitor;

impl<'de> Visitor<'de> for UniqueValueVisitor {
    type Value = UniqueValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON with unique object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(|number| UniqueValue(Value::Number(number)))
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.visit_unit()
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element::<UniqueValue>()? {
            values.push(value.0);
        }
        Ok(UniqueValue(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if values.contains_key(&key) {
                // Line/column identify the bad input without echoing an
                // unbounded or potentially sensitive key in diagnostics.
                return Err(de::Error::custom(
                    "duplicate object key; remove duplicates before loading",
                ));
            }
            let value = access.next_value::<UniqueValue>()?;
            values.insert(key, value.0);
        }
        Ok(UniqueValue(Value::Object(values)))
    }
}

pub(super) fn parse_project_json(json: &str) -> Result<Value, String> {
    parse_json_with_byte_limit(json, PROJECT_FILE_MAX_BYTES, "Project JSON")
}

pub(super) fn parse_json_with_byte_limit(
    json: &str,
    limit: u64,
    label: &str,
) -> Result<Value, String> {
    if json.len() as u64 > limit {
        return Err(format!(
            "{label} is {} bytes; the limit is {limit} bytes",
            json.len()
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let value = UniqueValue::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(value.0)
}

pub(super) fn parse_project_json_bytes(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() as u64 > PROJECT_FILE_MAX_BYTES {
        return Err(format!(
            "Project JSON is {} bytes; the limit is {PROJECT_FILE_MAX_BYTES} bytes",
            bytes.len()
        ));
    }
    let json = std::str::from_utf8(bytes)
        .map_err(|error| format!("Project JSON is not valid UTF-8: {error}"))?;
    parse_project_json(json)
}

fn read_bounded_project_bytes(
    reader: impl Read,
    declared_size: u64,
    limit: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    if declared_size > limit {
        return Err(format!(
            "{label} is {declared_size} bytes; the limit is {limit} bytes"
        ));
    }
    // Metadata is only a preflight optimization. The same opened handle may
    // grow while read; consume at most one byte beyond the actual size limit.
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{label} exceeds the limit of {limit} bytes while reading"
        ));
    }
    Ok(bytes)
}

pub(super) fn read_project_bytes(path: &Path) -> Result<Vec<u8>, String> {
    read_json_bytes_with_byte_limit(path, PROJECT_FILE_MAX_BYTES, "Project file")
}

fn read_json_bytes_with_byte_limit(
    path: &Path,
    limit: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    read_bounded_project_bytes(file, size, limit, label)
}

pub(super) fn read_json_with_byte_limit(
    path: &Path,
    limit: u64,
    label: &str,
) -> Result<String, String> {
    String::from_utf8(read_json_bytes_with_byte_limit(path, limit, label)?)
        .map_err(|error| format!("{label} is not valid UTF-8: {error}"))
}

pub(super) fn read_project_json(path: &Path) -> Result<String, String> {
    String::from_utf8(read_project_bytes(path)?)
        .map_err(|error| format!("Project JSON is not valid UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, io::Cursor};

    #[test]
    fn project_json_unambiguous_values_match_serde() {
        for json in [
            crate::PHASE1_SAMPLE_PROJECT_JSON,
            r#"{"n":null,"t":true,"f":false,"s":"日本語 🎚","i":-9223372036854775808,"u":18446744073709551615,"d":1.25e20,"a":[0,{},[]]}"#,
            "null",
            "false",
            "42",
            "-0.0",
            r#""text""#,
        ] {
            assert_eq!(
                parse_project_json(json).unwrap(),
                serde_json::from_str::<Value>(json).unwrap()
            );
        }
    }

    #[test]
    fn project_json_rejects_root_nested_and_escaped_duplicate_keys() {
        for json in [
            r#"{"version":0,"version":1}"#,
            r#"{"version":1,"version":0}"#,
            r#"{"version":1,"version":1}"#,
            r#"{"version":1,"ver\u0073ion":1}"#,
            r#"{"snapshot":{"clock":{"bpm":97,"bpm":120}}}"#,
            r#"{"future":[{"x":0,"x":1}]}"#,
        ] {
            let error = parse_project_json(json).unwrap_err();
            assert!(error.contains("duplicate object key"), "{error}");
            assert!(error.contains("line 1 column"), "{error}");
        }
    }

    #[test]
    fn project_json_allows_sibling_keys_case_and_string_contents() {
        let json = r#"{"a":{"x":1},"b":{"x":2},"Version":1,"version":1,"s":"{\"x\":0,\"x\":1}"}"#;
        assert_eq!(
            parse_project_json(json).unwrap(),
            serde_json::from_str::<Value>(json).unwrap()
        );
    }

    #[test]
    fn project_json_rejects_invalid_utf8_numbers_trailing_data_and_depth() {
        assert!(parse_project_json_bytes(b"{\"x\":\"\xff\"}")
            .unwrap_err()
            .contains("UTF-8"));
        for json in [
            "",
            "{",
            "{} {}",
            "{\"n\":NaN}",
            "{\"n\":Infinity}",
            "{\"n\":1e9999}",
            "{\"x\":0,}",
        ] {
            assert!(parse_project_json(json).is_err(), "accepted {json}");
        }
        for depth in [128, 256, 512] {
            assert!(
                parse_project_json(&format!("{}0{}", "[".repeat(depth), "]".repeat(depth)))
                    .is_err()
            );
        }
        assert_eq!(
            parse_project_json("{} \n\t").unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn project_json_size_boundary_is_the_existing_64_mib_limit() {
        let mut json = " ".repeat(PROJECT_FILE_MAX_BYTES as usize - 2);
        json.push_str("{}");
        assert!(parse_project_json(&json).is_ok());
        json.push(' ');
        assert!(parse_project_json(&json).unwrap_err().contains("limit"));
    }

    #[test]
    fn project_json_reader_bounds_growth_after_metadata() {
        struct CountingReader<'a> {
            consumed: &'a Cell<usize>,
            remaining: usize,
        }
        impl Read for CountingReader<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let count = buffer.len().min(self.remaining);
                buffer[..count].fill(b' ');
                self.remaining -= count;
                self.consumed.set(self.consumed.get() + count);
                Ok(count)
            }
        }
        let consumed = Cell::new(0);
        let error = read_bounded_project_bytes(
            CountingReader {
                consumed: &consumed,
                remaining: 100000,
            },
            2,
            16,
            "Project file",
        )
        .unwrap_err();
        assert!(error.contains("while reading"));
        assert_eq!(
            consumed.get(),
            17,
            "growing source must not be drained past limit+1"
        );
        assert_eq!(
            read_bounded_project_bytes(Cursor::new(b"{}"), 16, 16, "Project file").unwrap(),
            b"{}"
        );
        assert_eq!(
            read_bounded_project_bytes(Cursor::new([b' '; 16]), 16, 16, "Project file")
                .unwrap()
                .len(),
            16
        );
    }

    #[test]
    fn project_json_oversized_metadata_is_rejected_before_read() {
        struct NeverRead;
        impl Read for NeverRead {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("oversized source must not be read")
            }
        }
        assert!(
            read_bounded_project_bytes(NeverRead, 17, 16, "Project file")
                .unwrap_err()
                .contains("limit")
        );
    }
}
