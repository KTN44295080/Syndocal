//! Read-only audit pages. Each source owns its lock and retention policy.
//! No receipt eviction, admission, output work or whole-history copies occur here.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const PAGE_SIZE: usize = 16;
const MAX: u64 = protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
pub(crate) const SOURCES: [&str; 6] = [
    "agent_authority",
    "output_lease",
    "project_file",
    "project_replacement",
    "safety",
    "output_control",
];

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Before {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_authority: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_lease: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_file: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_replacement: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_control: Option<u64>,
}

// One DTO for native wire admission and immutable native execution. Omitted
// optional fields retain the legacy destination-only command/hash shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ExportRequest {
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_before: Option<Before>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_process_incarnation: Option<u64>,
}
impl Before {
    pub(crate) fn validate_shape(&self, expected: Option<u64>) -> Result<(), &'static str> {
        let values = self.values();
        if values.iter().flatten().any(|id| *id == 0 || *id > MAX)
            || expected.is_some_and(|id| id == 0 || id > MAX)
            || (values.iter().any(Option::is_some) && expected.is_none())
        {
            return Err("invalid_diagnostic_audit_cursor");
        }
        Ok(())
    }
    fn values(&self) -> [Option<u64>; 6] {
        [
            self.agent_authority,
            self.output_lease,
            self.project_file,
            self.project_replacement,
            self.safety,
            self.output_control,
        ]
    }
    pub(crate) fn validate(
        &self,
        expected_process: Option<u64>,
        current: u64,
    ) -> Result<(), String> {
        if self.validate_shape(expected_process).is_err()
            || current == 0
            || current > MAX
            || expected_process.is_some_and(|id| id != current)
        {
            return Err("diagnostic_audit_cursor_invalid_or_process_changed; observe the current process and use explicit source cursors".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Row {
    pub sequence: u64,
    pub phase: &'static str,
    pub actor_sha256: Option<String>,
    pub operation_sha256: Option<String>,
    pub request_sha256: Option<String>,
    pub shape_sha256: Option<String>,
    pub argument_sha256: Option<String>,
    pub event_sha256: Option<String>,
    pub outcome_sha256: Option<String>,
    pub succeeded: Option<bool>,
    pub generation_before: Option<u64>,
    pub generation_after: Option<u64>,
}
impl Row {
    pub(crate) fn new(
        sequence: u64,
        phase: &'static str,
        actor: Option<&str>,
        operation: Option<&str>,
        request: Option<u64>,
    ) -> Self {
        Self {
            sequence,
            phase,
            actor_sha256: actor.map(identity_hash),
            operation_sha256: operation.map(identity_hash),
            request_sha256: request.map(|id| identity_hash(&id.to_string())),
            shape_sha256: None,
            argument_sha256: None,
            event_sha256: None,
            outcome_sha256: None,
            succeeded: None,
            generation_before: None,
            generation_after: None,
        }
    }
}
pub(crate) fn identity_hash(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"syndocal-audit-sha256-v1\0");
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("String formatting is infallible");
    }
    text
}

#[derive(Debug, Serialize)]
pub(crate) struct Page {
    pub source: &'static str,
    pub retained_count: u64,
    pub retained_first_sequence: Option<u64>,
    pub retained_last_sequence: Option<u64>,
    pub selected_before_sequence: Option<u64>,
    pub has_expired_history: bool,
    pub next_before_sequence: Option<u64>,
    pub records: Vec<Row>,
}

// Inspect metadata, then project only the selected page. In particular, do not
// clone the 65,536-row file/replacement/runtime histories or their result bodies.
pub(crate) fn page<I, F, P>(
    source: &'static str,
    records: I,
    before: Option<u64>,
    sequence: F,
    project: P,
) -> Result<Page, String>
where
    I: Clone + DoubleEndedIterator + ExactSizeIterator,
    F: Fn(&I::Item) -> u64,
    P: Fn(I::Item, u64) -> Row,
{
    let count = records.len();
    let first = records.clone().next().map(|row| sequence(&row));
    let last = records.clone().next_back().map(|row| sequence(&row));
    if !SOURCES.contains(&source)
        || last.is_some_and(|id| id == 0 || id > MAX)
        || before.is_some_and(|id| {
            id == 0 || id > MAX || last.is_none_or(|last| id > last.saturating_add(1))
        })
    {
        return Err(
            "diagnostic_audit_cursor_ahead_or_invalid; observe the current source range".into(),
        );
    }
    if before.is_some_and(|cursor| first.is_some_and(|first| first > 1 && cursor <= first)) {
        return Err("diagnostic_audit_retention_expired; preserve prior pages and resnapshot the current retained range".into());
    }
    let mut selected = Vec::with_capacity(PAGE_SIZE);
    let mut more = false;
    for record in records.rev() {
        let id = sequence(&record);
        if before.is_some_and(|cursor| id >= cursor) {
            continue;
        }
        if selected.len() == PAGE_SIZE {
            more = true;
            break;
        }
        selected.push(project(record, id));
    }
    selected.reverse();
    let next_before_sequence = if more {
        selected.first().map(|row| row.sequence)
    } else {
        None
    };
    Ok(Page {
        source,
        retained_count: count as u64,
        retained_first_sequence: first,
        retained_last_sequence: last,
        selected_before_sequence: before,
        has_expired_history: first.is_some_and(|id| id > 1),
        next_before_sequence,
        records: selected,
    })
}

#[derive(Serialize)]
pub(crate) struct History {
    schema_version: u16,
    process_incarnation: u64,
    independent_source_observations: bool,
    full_attempt_fields_available: bool,
    pages: Vec<Page>,
}
impl History {
    pub(crate) fn summary(&self) -> serde_json::Value {
        serde_json::json!({"schema_version":1,"process_incarnation":self.process_incarnation,
            "independent_source_observations":true,"full_attempt_fields_available":false,
            "pages":self.pages.iter().map(|p|serde_json::json!({"source":p.source,"retained_count":p.retained_count,
                "retained_first_sequence":p.retained_first_sequence,"retained_last_sequence":p.retained_last_sequence,
                "selected_before_sequence":p.selected_before_sequence,"has_expired_history":p.has_expired_history,
                "next_before_sequence":p.next_before_sequence,"exported_count":p.records.len()})).collect::<Vec<_>>()})
    }
}
pub(crate) fn history(process: u64, pages: Vec<Page>) -> History {
    History {
        schema_version: 1,
        process_incarnation: process,
        independent_source_observations: true,
        full_attempt_fields_available: false,
        pages,
    }
}

// Called after the ZIP's field projection. Presence/type/hash rules belong to
// that schema; these relational checks prevent plausible but contradictory pages.
pub(crate) fn validate_history(value: &serde_json::Value) -> bool {
    use serde_json::Value;
    let optional = |v: &Value| {
        if v.is_null() {
            Some(None)
        } else {
            v.as_u64().filter(|v| *v > 0 && *v <= MAX).map(Some)
        }
    };
    if value["schema_version"] != 1
        || value["independent_source_observations"] != true
        || value["full_attempt_fields_available"] != false
        || value["process_incarnation"]
            .as_u64()
            .is_none_or(|p| p == 0 || p > MAX)
    {
        return false;
    }
    let Some(pages) = value["pages"]
        .as_array()
        .filter(|p| p.len() == SOURCES.len())
    else {
        return false;
    };
    for (index, p) in pages.iter().enumerate() {
        if p["source"] != SOURCES[index] {
            return false;
        }
        let (
            Some(count),
            Some(first),
            Some(last),
            Some(before),
            Some(next),
            Some(expired),
            Some(rows),
        ) = (
            p["retained_count"].as_u64(),
            optional(&p["retained_first_sequence"]),
            optional(&p["retained_last_sequence"]),
            optional(&p["selected_before_sequence"]),
            optional(&p["next_before_sequence"]),
            p["has_expired_history"].as_bool(),
            p["records"].as_array(),
        )
        else {
            return false;
        };
        if count > 65_536
            || rows.len() > PAGE_SIZE
            || rows.len() as u64 > count
            || (count == 0) != (first.is_none() && last.is_none())
        {
            return false;
        }
        if count > 0
            && first
                .zip(last)
                .is_none_or(|(a, b)| a > b || count > b - a + 1)
        {
            return false;
        }
        if expired != first.is_some_and(|id| id > 1)
            || before.is_some_and(|id| {
                last.is_none_or(|last| id > last + 1) || first.is_some_and(|f| f > 1 && id <= f)
            })
        {
            return false;
        }
        let mut prior = 0;
        for row in rows {
            let Some(id) = row["sequence"]
                .as_u64()
                .filter(|id| *id > prior && *id <= MAX)
            else {
                return false;
            };
            if first.is_none_or(|f| id < f)
                || last.is_none_or(|l| id > l)
                || before.is_some_and(|cursor| id >= cursor)
            {
                return false;
            }
            for key in ["generation_before", "generation_after"] {
                if !row[key].is_null() && row[key].as_u64().is_none_or(|id| id > MAX) {
                    return false;
                }
            }
            prior = id;
        }
        if next.is_some_and(|id| {
            rows.first().and_then(|r| r["sequence"].as_u64()) != Some(id)
                || first.is_none_or(|f| id <= f)
        }) {
            return false;
        }
        let earliest_selected = rows.first().and_then(|r| r["sequence"].as_u64());
        if next.is_some()
            != earliest_selected
                .zip(first)
                .is_some_and(|(selected, retained)| selected > retained)
            || (next.is_some() && rows.len() != PAGE_SIZE)
            || (rows.is_empty() && before.is_some_and(|cursor| first.is_some_and(|f| cursor > f)))
            || (before.is_none() && rows.last().and_then(|r| r["sequence"].as_u64()) != last)
        {
            return false;
        }
        if before.is_none() && (next.is_some() != (count > rows.len() as u64)) {
            return false;
        }
    }
    true
}

#[cfg(test)]
pub(crate) fn fixture_history() -> serde_json::Value {
    serde_json::to_value(history(
        1,
        SOURCES
            .iter()
            .map(|source| {
                page(
                    source,
                    [0_u64; 0].iter(),
                    None,
                    |v| **v,
                    |_, id| Row::new(id, "event", None, None, None),
                )
                .unwrap()
            })
            .collect(),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_audit_pages_cover_retained_history_without_duplicates_or_implicit_mutation() {
        let original: Vec<u64> = (1..=41).collect();
        let mut before = None;
        let mut all = Vec::new();
        loop {
            let p = page(
                "project_file",
                original.iter(),
                before,
                |id| **id,
                |id, seq| Row::new(seq, "admitted", Some("PRIVATE"), None, Some(*id)),
            )
            .unwrap();
            assert_eq!(p.retained_count, 41);
            assert!(!p.has_expired_history);
            assert!(p.records.len() <= PAGE_SIZE);
            all.extend(p.records.iter().map(|row| row.sequence));
            before = p.next_before_sequence;
            if before.is_none() {
                break;
            }
        }
        all.sort_unstable();
        assert_eq!(all, original);
        assert_eq!(original, (1..=41).collect::<Vec<_>>());
    }
    #[test]
    fn diagnostic_audit_process_future_and_expired_cursors_reject_without_fabricated_empty_pages() {
        let records: Vec<u64> = (12..=32).collect();
        for before in [Some(0), Some(12), Some(34), Some(MAX + 1)] {
            assert!(page(
                "agent_authority",
                records.iter(),
                before,
                |v| **v,
                |_, id| Row::new(id, "event", None, None, None)
            )
            .is_err());
        }
        let mut cursor = Before::default();
        cursor.agent_authority = Some(20);
        assert!(cursor.validate(None, 7).is_err());
        assert!(cursor.validate(Some(8), 7).is_err());
        assert!(cursor.validate(Some(7), 7).is_ok());
        assert!(serde_json::from_str::<Before>(r#"{"owner":"forged"}"#).is_err());
    }
    #[test]
    fn diagnostic_audit_projection_reads_only_selected_rows_and_never_emits_identity_text() {
        use std::cell::Cell;
        let count = Cell::new(0);
        let records: Vec<u64> = (1..=65_536).collect();
        let page = page(
            "safety",
            records.iter(),
            None,
            |v| **v,
            |_, id| {
                count.set(count.get() + 1);
                Row::new(
                    id,
                    "admitted",
                    Some("PRIVATE_PATH"),
                    Some("PRIVATE_OPERATION"),
                    Some(id),
                )
            },
        )
        .unwrap();
        assert_eq!(count.get(), PAGE_SIZE);
        assert_eq!(page.next_before_sequence, Some(65_521));
        let bytes = serde_json::to_string(&page).unwrap();
        assert!(!bytes.contains("PRIVATE"));
        assert!(bytes.len() < 16 * 1024);
    }
}
