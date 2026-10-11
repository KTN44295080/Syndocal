use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FixturePatchChange {
    fixture_id: u64,
    expected_universe: u16,
    expected_address: u16,
    expected_footprint: u16,
    label: String,
    universe: u16,
    address: u16,
}
impl FixturePatchChange {
    pub(crate) fn into_engine(self) -> Result<engine::FixturePatchChange, String> {
        if self.fixture_id == 0
            || self.fixture_id > protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err("Fixture move ID must be non-zero and JavaScript-safe".into());
        }
        Ok(engine::FixturePatchChange {
            fixture_id: self.fixture_id,
            expected_universe: self.expected_universe,
            expected_address: self.expected_address,
            expected_footprint: self.expected_footprint,
            label: self.label,
            universe: self.universe,
            address: self.address,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> serde_json::Value {
        serde_json::json!({"fixtureId":7,"expectedUniverse":1,"expectedAddress":1,"expectedFootprint":3,"label":"Front","universe":2,"address":50})
    }
    #[test]
    fn exact_move_request_rejects_unknown_fields_and_unsafe_ids() {
        serde_json::from_value::<FixturePatchChange>(valid())
            .unwrap()
            .into_engine()
            .unwrap();
        let mut future = valid();
        future["future"] = true.into();
        assert!(serde_json::from_value::<FixturePatchChange>(future).is_err());
        for id in [
            0,
            protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER + 1,
        ] {
            let mut request = valid();
            request["fixtureId"] = id.into();
            assert!(serde_json::from_value::<FixturePatchChange>(request)
                .unwrap()
                .into_engine()
                .is_err());
        }
    }
}
