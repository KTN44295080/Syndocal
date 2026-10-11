//! Readdress existing fixtures as one validated publication. Moving IDs are
//! removed from A's occupied ranges together before validating every B range.
use super::*;

#[derive(Debug, Clone)]
pub struct FixturePatchChange {
    pub fixture_id: FixtureId,
    pub expected_universe: u16,
    pub expected_address: u16,
    pub expected_footprint: u16,
    pub label: String,
    pub universe: u16,
    pub address: u16,
}

pub(crate) fn apply(
    fixtures: &mut [RuntimeFixture],
    changes: Vec<FixturePatchChange>,
) -> Result<(), String> {
    if changes.is_empty() || changes.len() > 256 {
        return Err("Fixture move requires from 1 to 256 fixtures".into());
    }
    let mut moving = HashSet::with_capacity(changes.len());
    for change in &changes {
        if change.fixture_id == 0 || !moving.insert(change.fixture_id) {
            return Err("Fixture move IDs must be unique and non-zero".into());
        }
    }
    let mut occupied = Vec::with_capacity(fixtures.len());
    for fixture in fixtures
        .iter()
        .filter(|fixture| !moving.contains(&fixture.id))
    {
        let footprint =
            runtime_fixture_footprint(fixture).ok_or("Existing fixture footprint is invalid")?;
        let range = runtime_dmx_range(fixture.request.address, footprint)
            .ok_or("Existing fixture address is invalid")?;
        occupied.push((fixture.id, fixture.request.universe, range));
    }
    let mut prepared = Vec::with_capacity(changes.len());
    for mut change in changes {
        change.label = change.label.trim().to_string();
        validate_runtime_fixture_patch(&change.label, change.address)?;
        let index = fixtures
            .iter()
            .position(|fixture| fixture.id == change.fixture_id)
            .ok_or_else(|| format!("Fixture {} was not found", change.fixture_id))?;
        let fixture = &fixtures[index];
        let footprint =
            runtime_fixture_footprint(fixture).ok_or("Fixture has no DMX channel offsets")?;
        if fixture.request.universe != change.expected_universe
            || fixture.request.address != change.expected_address
            || footprint != change.expected_footprint
        {
            return Err(format!(
                "Fixture {} patch changed during the move; nothing was applied",
                change.fixture_id
            ));
        }
        let range = runtime_dmx_range(change.address, footprint)
            .ok_or("Fixture has no DMX channel offsets")?;
        if range.1 > 512 {
            return Err(format!(
                "Fixture footprint exceeds DMX universe: end {}",
                range.1
            ));
        }
        if let Some((id, _, existing)) = occupied.iter().find(|(_, universe, existing)| {
            *universe == change.universe && range.0 <= existing.1 && existing.0 <= range.1
        }) {
            return Err(format!(
                "DMX address conflict in universe {}: fixture {} at {}-{} overlaps fixture {id} at {}-{}; nothing was applied",
                change.universe, change.fixture_id, range.0, range.1, existing.0, existing.1
            ));
        }
        occupied.push((change.fixture_id, change.universe, range));
        prepared.push((index, change));
    }
    for (index, change) in prepared {
        fixtures[index].request.label = change.label;
        fixtures[index].request.universe = change.universe;
        fixtures[index].request.address = change.address;
    }
    Ok(())
}
