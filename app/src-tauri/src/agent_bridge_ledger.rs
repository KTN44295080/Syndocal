//! Bounded request identity and renderer incarnation authority. No engine access.
use super::{
    wire::{canonical_operation_is_mutation, Command, Response},
    AgentBridgeDispatch,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
const DETAIL_LIMIT: usize = 64;
const MUTATION_ID_LIMIT: usize = 4096;
const MAX_LEDGER_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutation {
    shape: String,
    response: Option<Response>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Durable {
    version: u32,
    mutations: BTreeMap<String, Mutation>,
}
struct Entry {
    shape: String,
    response: Response,
    dispatch: AgentBridgeDispatch,
    claimed: bool,
    mutation: bool,
}
pub(super) struct Ledger {
    generation: u64,
    available: bool,
    entries: BTreeMap<String, Entry>,
    durable: Durable,
    path: Option<PathBuf>,
}
impl Ledger {
    pub fn new(path: Option<PathBuf>) -> Result<Self, String> {
        let mut durable = Durable {
            version: 1,
            ..Durable::default()
        };
        if let Some(path) = &path {
            match std::fs::metadata(path) {
                Ok(meta) => {
                    if meta.len() > MAX_LEDGER_BYTES {
                        return Err("ledger_oversize".to_string());
                    }
                    use std::io::Read;
                    let mut bytes = Vec::new();
                    std::fs::File::open(path)
                        .map_err(|_| "ledger_read_failed")?
                        .take(MAX_LEDGER_BYTES + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|_| "ledger_read_failed")?;
                    if bytes.len() as u64 > MAX_LEDGER_BYTES {
                        return Err("ledger_oversize".to_string());
                    }
                    durable = serde_json::from_slice(&bytes).map_err(|_| "ledger_invalid")?;
                    if durable.version != 1 || durable.mutations.len() > MUTATION_ID_LIMIT {
                        return Err("ledger_invalid".to_string());
                    }
                    for (id, value) in &mut durable.mutations {
                        if !super::wire::valid_uuid(id)
                            || value.shape.len() != 64
                            || !value.shape.bytes().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("ledger_invalid".to_string());
                        }
                        // A previous process cannot establish this renderer's completion.
                        value.response = None;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("ledger_read_failed".to_string()),
            }
        }
        let ledger = Self {
            generation: 0,
            available: false,
            entries: BTreeMap::new(),
            durable,
            path,
        };
        ledger.persist(&ledger.durable)?;
        Ok(ledger)
    }
    fn persist(&self, candidate: &Durable) -> Result<(), String> {
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec(candidate).map_err(|_| "ledger_encode_failed")?;
            if bytes.len() as u64 > MAX_LEDGER_BYTES {
                return Err("ledger_oversize".to_string());
            }
            super::storage::atomic_write(path, &bytes)?;
        }
        Ok(())
    }
    pub fn register(&mut self) -> Result<u64, String> {
        self.available = false;
        let next = self
            .generation
            .checked_add(1)
            .filter(|value| *value <= 9_007_199_254_740_991)
            .ok_or("renderer_generation_exhausted")?;
        let mut candidate = self.durable.clone();
        for (id, entry) in &mut self.entries {
            if entry.response.status == "pending" {
                entry.response = Response::status(id, "unknown");
                if let Some(record) = candidate.mutations.get_mut(id) {
                    record.response = Some(entry.response.clone());
                }
            }
        }
        self.persist(&candidate)?;
        self.durable = candidate;
        self.generation = next;
        self.available = true;
        Ok(next)
    }
    pub fn status(&self, id: &str) -> Response {
        self.entries
            .get(id)
            .map(|entry| entry.response.clone())
            .or_else(|| {
                self.durable
                    .mutations
                    .get(id)
                    .and_then(|value| value.response.clone())
            })
            .unwrap_or_else(|| Response::status(id, "unknown"))
    }
    pub fn begin(
        &mut self,
        id: &str,
        command: &Command,
    ) -> Result<(Response, Option<AgentBridgeDispatch>), String> {
        use sha2::{Digest, Sha256};
        let encoded = serde_json::to_vec(command).map_err(|_| "request_encode_failed")?;
        let shape = format!("{:x}", Sha256::digest(encoded));
        if let Some(entry) = self.entries.get(id) {
            return Ok((
                if entry.shape == shape {
                    entry.response.clone()
                } else {
                    Response::rejected(id, "request_conflict")
                },
                None,
            ));
        }
        if let Some(record) = self.durable.mutations.get(id) {
            return Ok((
                if record.shape == shape {
                    self.status(id)
                } else {
                    Response::rejected(id, "request_conflict")
                },
                None,
            ));
        }
        if !self.available {
            return Ok((Response::rejected(id, "not_available"), None));
        }
        let mutation = match command {
            Command::SetTransform(_) | Command::SetVideoBlackout(_) => true,
            Command::ControlPlaneExecute(value) => {
                canonical_operation_is_mutation(&value.operation_id)
            }
            _ => false,
        };
        if mutation && self.durable.mutations.len() == MUTATION_ID_LIMIT {
            return Ok((Response::rejected(id, "ledger_capacity"), None));
        }
        let evict = if self.entries.len() == DETAIL_LIMIT {
            Some(
                self.entries
                    .iter()
                    .find(|(_, value)| value.response.status != "pending")
                    .map(|(id, _)| id.clone())
                    .ok_or("inflight_capacity")?,
            )
        } else {
            None
        };
        let command_json = serde_json::to_value(command).map_err(|_| "request_encode_failed")?;
        let dispatch = AgentBridgeDispatch {
            renderer_generation: self.generation,
            request_id: id.to_string(),
            method: command_json["method"]
                .as_str()
                .ok_or("request_encode_failed")?
                .to_string(),
            params: command_json["params"].clone(),
            principal_id: String::new(),
            principal_incarnation: 0,
        };
        let response = Response::status(id, "pending");
        let mut candidate = self.durable.clone();
        if let Some(evict) = &evict {
            if let Some(record) = candidate.mutations.get_mut(evict) {
                record.response = None;
            }
        }
        if mutation {
            candidate.mutations.insert(
                id.to_string(),
                Mutation {
                    shape: shape.clone(),
                    response: Some(response.clone()),
                },
            );
        }
        if mutation
            || evict
                .as_ref()
                .is_some_and(|id| self.durable.mutations.contains_key(id))
        {
            self.persist(&candidate)?;
        }
        self.durable = candidate;
        if let Some(evict) = evict {
            self.entries.remove(&evict);
        }
        self.entries.insert(
            id.to_string(),
            Entry {
                shape,
                response: response.clone(),
                dispatch: dispatch.clone(),
                claimed: false,
                mutation,
            },
        );
        Ok((response, Some(dispatch)))
    }
    pub fn claim(&mut self, generation: u64, id: &str) -> Result<AgentBridgeDispatch, String> {
        if !self.available || generation != self.generation {
            return Err("stale_renderer".to_string());
        }
        let entry = self.entries.get_mut(id).ok_or("unknown_request")?;
        if entry.dispatch.renderer_generation != generation
            || entry.claimed
            || entry.response.status != "pending"
        {
            return Err("request_not_claimable".to_string());
        }
        entry.claimed = true;
        Ok(entry.dispatch.clone())
    }
    pub fn complete(
        &mut self,
        generation: u64,
        id: &str,
        result: serde_json::Value,
    ) -> Result<(), String> {
        if !self.available || generation != self.generation {
            return Err("stale_renderer".to_string());
        }
        if serde_json::to_vec(&result)
            .map_err(|_| "result_invalid")?
            .len()
            > super::wire::MAX_RESULT_BYTES
        {
            return Err("result_oversize".to_string());
        }
        let entry = self.entries.get(id).ok_or("unknown_request")?;
        if entry.dispatch.renderer_generation != generation
            || !entry.claimed
            || entry.response.status != "pending"
        {
            return Err("request_not_completable".to_string());
        }
        let mut response = Response::status(id, "completed");
        response.result = Some(result);
        if entry.mutation {
            let mut candidate = self.durable.clone();
            candidate
                .mutations
                .get_mut(id)
                .ok_or("ledger_identity_missing")?
                .response = Some(response.clone());
            if let Err(error) = self.persist(&candidate) {
                self.available = false;
                self.entries.get_mut(id).unwrap().response = Response::status(id, "unknown");
                return Err(error);
            }
            self.durable = candidate;
        }
        self.entries.get_mut(id).unwrap().response = response;
        Ok(())
    }
    pub fn dispatch_failed(&mut self, id: &str) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.response = Response::status(id, "unknown");
        }
        // Durable pending already prevents restart replay; no proof of non-application.
    }
}
