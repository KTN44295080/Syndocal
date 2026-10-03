//! Bounded returned diagnostics, independent of engine/publication ownership.
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Write};

pub(super) const MAX_INPUT_DIAGNOSTIC_BYTES: usize = 1024;
const TRUNCATED: &str = " ... [truncated]";

struct DiagnosticWriter {
    text: String,
    truncated: bool,
}

impl Write for DiagnosticWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for character in value.chars() {
            let mut encoded = [0; 4];
            if character.is_control() {
                for escaped in character.escape_default() {
                    self.append(escaped.encode_utf8(&mut encoded))?;
                }
            } else {
                self.append(character.encode_utf8(&mut encoded))?;
            }
        }
        Ok(())
    }
}

impl DiagnosticWriter {
    fn append(&mut self, value: &str) -> fmt::Result {
        if self.text.len() + value.len() > MAX_INPUT_DIAGNOSTIC_BYTES {
            self.truncated = true;
            return Err(fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}

pub(super) fn bounded_diagnostic(value: impl fmt::Display) -> String {
    let mut writer = DiagnosticWriter {
        text: String::with_capacity(MAX_INPUT_DIAGNOSTIC_BYTES),
        truncated: false,
    };
    let _ = write!(&mut writer, "{value}");
    if writer.truncated {
        let mut end = MAX_INPUT_DIAGNOSTIC_BYTES - TRUNCATED.len();
        end = end.min(writer.text.len());
        while !writer.text.is_char_boundary(end) {
            end -= 1;
        }
        writer.text.truncate(end);
        writer.text.push_str(TRUNCATED);
    }
    if writer.text.is_empty() {
        "Invalid input; check its format before loading".to_string()
    } else {
        writer.text
    }
}

pub(super) fn deserialize_input<'de, D, T>(deserializer: D, label: &str) -> Result<T, String>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    serde_path_to_error::deserialize(deserializer).map_err(|error| {
        // Serde errors can contain the entire failing string. Keep the field
        // location, never the error's value/variant text. Byte parsing reports
        // syntax and line/column separately before this schema boundary.
        bounded_diagnostic(format_args!(
            "Invalid {label} schema at {}: unsupported value, wrong type or missing required field; check this field's format",
            DiagnosticPath(error.path())
        ))
    })
}

struct DiagnosticPath<'a>(&'a serde_path_to_error::Path);

impl fmt::Display for DiagnosticPath<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for segment in self.0 {
            use serde_path_to_error::Segment;
            match segment {
                Segment::Seq { index } => write!(formatter, "[{index}]")?,
                Segment::Map { key } => {
                    formatter.write_str(separator)?;
                    if key.len() <= 64
                        && !key.is_empty()
                        && key
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '_')
                    {
                        formatter.write_str(key)?;
                    } else {
                        formatter.write_str("[map-key]")?;
                    }
                }
                Segment::Enum { .. } => write!(formatter, "{separator}<variant>")?,
                Segment::Unknown => write!(formatter, "{separator}[unknown-field]")?,
            }
            separator = ".";
        }
        if separator.is_empty() {
            formatter.write_str("[root]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;

    #[test]
    fn input_diagnostic_preserves_short_unicode_and_exact_byte_limit() {
        assert_eq!(bounded_diagnostic("日本語 🎚"), "日本語 🎚");
        assert_eq!(
            bounded_diagnostic("a".repeat(MAX_INPUT_DIAGNOSTIC_BYTES)),
            "a".repeat(MAX_INPUT_DIAGNOSTIC_BYTES)
        );
        for value in ["a".repeat(2048), "漢😀".repeat(2048)] {
            let message = bounded_diagnostic(&value);
            assert!(message.len() <= MAX_INPUT_DIAGNOSTIC_BYTES);
            assert!(message.ends_with(TRUNCATED));
        }
    }

    #[test]
    fn input_diagnostic_escapes_control_characters_before_bounding() {
        let message = bounded_diagnostic("a\n\t\u{1b}\u{7f}\u{85}b");
        assert!(!message.chars().any(char::is_control));
        assert!(message.contains("\\n") && message.contains("\\t"));
        assert!(bounded_diagnostic("\n".repeat(2048)).len() <= MAX_INPUT_DIAGNOSTIC_BYTES);
    }

    #[test]
    fn input_diagnostic_stops_formatting_when_full() {
        struct Streaming<'a>(&'a Cell<usize>);
        impl fmt::Display for Streaming<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..100_000 {
                    self.0.set(self.0.get() + 1);
                    formatter.write_str("x")?;
                }
                Ok(())
            }
        }
        let writes = Cell::new(0);
        assert_eq!(
            bounded_diagnostic(Streaming(&writes)).len(),
            MAX_INPUT_DIAGNOSTIC_BYTES
        );
        assert_eq!(writes.get(), MAX_INPUT_DIAGNOSTIC_BYTES + 1);
    }

    #[test]
    fn input_diagnostic_schema_redacts_failing_values_and_locates_fields() {
        let mut value: serde_json::Value =
            serde_json::from_str(crate::PHASE1_SAMPLE_PROJECT_JSON).unwrap();
        let secret = format!("PRIVATE_VALUE_MUST_NOT_BE_ECHOED{}", "漢😀".repeat(8192));
        for (field, replacement) in [("protocol", secret.clone()), ("bpm", secret)] {
            let (section, expected) = if field == "protocol" {
                ("output", "snapshot.output.protocol")
            } else {
                ("clock", "snapshot.clock.bpm")
            };
            let original = value["snapshot"][section][field].take();
            value["snapshot"][section][field] = json!(replacement);
            let error = deserialize_input::<_, crate::ProjectFile>(&value, "Project JSON")
                .err()
                .unwrap();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("PRIVATE_VALUE_MUST_NOT_BE_ECHOED"));
            assert!(error.len() <= MAX_INPUT_DIAGNOSTIC_BYTES);
            value["snapshot"][section][field] = original;
        }
        let decoded: crate::ProjectFile = deserialize_input(value, "Project JSON").unwrap();
        assert_eq!(decoded.app, crate::APP_NAME);
    }

    #[test]
    fn input_diagnostic_bounds_dynamic_map_keys_and_omits_enum_variants() {
        let key = format!("PRIVATE_KEY{}\n", "漢😀".repeat(8192));
        let value = json!({key: "PRIVATE_VALUE"});
        let error =
            deserialize_input::<_, std::collections::BTreeMap<String, u32>>(value, "Map JSON")
                .err()
                .unwrap();
        assert!(error.contains("[map-key]"));
        assert!(!error.contains("PRIVATE_"));
        assert!(!error.chars().any(char::is_control));
        #[derive(Deserialize)]
        enum Example {
            PrivateVariant(u32),
        }
        let result =
            deserialize_input::<_, Example>(json!({"PrivateVariant":"PRIVATE_VALUE"}), "Enum JSON");
        let error = match result {
            Err(error) => error,
            Ok(Example::PrivateVariant(value)) => panic!("unexpected value {value}"),
        };
        assert!(error.contains("<variant>"), "{error}");
        assert!(!error.contains("PrivateVariant") && !error.contains("PRIVATE_VALUE"));
    }

    #[test]
    fn input_diagnostic_foreign_app_rejection_does_not_echo_input() {
        let app = format!("PRIVATE_VALUE\n{}", "漢😀".repeat(8192));
        let error = crate::validate_app_name("project", &app).unwrap_err();
        assert!(error.contains("Unsupported project app"));
        assert!(!error.contains("PRIVATE_VALUE"));
        assert!(error.len() <= MAX_INPUT_DIAGNOSTIC_BYTES);
        assert!(crate::validate_app_name("project", &format!(" {} ", crate::APP_NAME)).is_ok());
    }
}
