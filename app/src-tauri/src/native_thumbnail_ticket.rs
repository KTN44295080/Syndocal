use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ThumbnailLane {
    Layer,
    Asset,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ThumbnailTicket {
    pub(crate) schema_version: u8,
    pub(crate) lane: ThumbnailLane,
    pub(crate) request_id: String,
}

impl ThumbnailTicket {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.request_id.len() != 32
            || !self
                .request_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Invalid thumbnail cancellation ticket".to_string());
        }
        Ok(())
    }
}
