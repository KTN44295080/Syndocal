//! Application-local DJ Link persistence authority.
//!
//! This module composes the strict machine settings/credential primitives and
//! NLM-backed wired discovery.  It owns no project state and never opens a
//! project: callers obtain a DJ-only listener config only after a complete
//! persisted binding, restored credential, two-pass network trust, and fresh
//! exact IPv4 tuple revalidation.

use crate::dj_link_machine::{
    dj_link_machine_settings_path, load_dj_link_machine_settings_from_path,
    persist_dj_link_machine_settings_to_path, platform_dj_link_credential_store,
    DjLinkCredentialStore, DjLinkMachineRollback, DjLinkMachineSettingsLoadOutcome,
    DjLinkMachineSettingsV2, DjLinkMachineTransactionState, DjLinkToken,
};
use crate::dj_link_network::{self, DjLinkIpv4BindingKey, DjLinkTrustDecision};
use base64::Engine as _;
use protocol::RemoteControlConfig;
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path, path::PathBuf, time::Duration};

const DJ_LINK_NETWORK_SETTLE_DWELL: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct DjLinkMachineAuthority {
    settings_path: Option<PathBuf>,
    settings: DjLinkMachineSettingsV2,
    /// Stable, non-secret operator-visible reason. Corrupt/future evidence
    /// remains untouched rather than being overwritten to clear this state.
    block_reason: Option<String>,
    restored_credential: bool,
    /// Some credential-record cleanup remains pending. After a stable rotate
    /// this is only the old rollback slot; during a disarm it may include the
    /// primary slot too. It never contains a secret or error detail.
    credential_cleanup_pending: bool,
    /// Process-only listener injection value. The only durable token bytes
    /// are in Credential Manager; settings do not contain this value or a
    /// derivative of it.
    wire_token: Option<String>,
}

/// `wire_token` is intentionally process-only, but it is still a secret and
/// must never reach diagnostic output through this authority.
impl fmt::Debug for DjLinkMachineAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DjLinkMachineAuthority")
            .field("settings_path", &self.settings_path)
            .field("settings", &self.settings)
            .field("block_reason", &self.block_reason)
            .field("restored_credential", &self.restored_credential)
            .field(
                "credential_cleanup_pending",
                &self.credential_cleanup_pending,
            )
            .field("wire_token", &"REDACTED")
            .finish()
    }
}

trait DjLinkSettingsPersistence {
    fn persist(&self, settings: &DjLinkMachineSettingsV2) -> Result<(), String>;
}

struct PathDjLinkSettingsPersistence {
    path: PathBuf,
}

impl DjLinkSettingsPersistence for PathDjLinkSettingsPersistence {
    fn persist(&self, settings: &DjLinkMachineSettingsV2) -> Result<(), String> {
        persist_dj_link_machine_settings_to_path(&self.path, settings)
    }
}

trait DjLinkTokenGenerator {
    fn generate(&self) -> Result<DjLinkToken, String>;
}

struct OsDjLinkTokenGenerator;

impl DjLinkTokenGenerator for OsDjLinkTokenGenerator {
    fn generate(&self) -> Result<DjLinkToken, String> {
        DjLinkToken::generate().map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DjLinkMachineStatus {
    pub configured: bool,
    pub credential_ready: bool,
    pub auto_start_armed: bool,
    pub bind_ip: Option<String>,
    pub bind_port: Option<u16>,
    pub network_guid: Option<String>,
    pub adapter_guid: Option<String>,
    pub credential_generation: Option<u64>,
    pub credential_cleanup_pending: bool,
    /// Never contains a token or token-derived value.
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DjLinkWiredCandidate {
    pub network_guid: String,
    pub adapter_guid: String,
    pub bind_ip: String,
    pub adapter_alias: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DjLinkMachineArmRequest {
    pub network_guid: String,
    pub adapter_guid: String,
    pub bind_ip: String,
    pub bind_port: u16,
}

impl DjLinkMachineAuthority {
    pub fn block(&mut self, reason: &'static str) {
        self.wire_token = None;
        self.restored_credential = false;
        self.block_reason = Some(reason.to_string());
    }

    pub fn status(&self) -> DjLinkMachineStatus {
        let settings = &self.settings;
        DjLinkMachineStatus {
            configured: settings.credential_generation.is_some()
                && settings.adapter_guid.is_some()
                && settings.network_guid.is_some()
                && settings.bind_ip.is_some()
                && settings.bind_port.is_some(),
            credential_ready: self.restored_credential,
            auto_start_armed: settings.auto_start_armed,
            bind_ip: settings.bind_ip.clone(),
            bind_port: settings.bind_port,
            network_guid: settings.network_guid.clone(),
            adapter_guid: settings.adapter_guid.clone(),
            credential_generation: settings.credential_generation,
            credential_cleanup_pending: self.credential_cleanup_pending,
            block_reason: self.block_reason.clone(),
        }
    }

    fn ready_settings(&self) -> Result<&DjLinkMachineSettingsV2, String> {
        if let Some(reason) = self.block_reason.as_deref() {
            return Err(format!("DJ Link machine authority is blocked: {reason}"));
        }
        if self.settings.transaction_state != DjLinkMachineTransactionState::Idle {
            return Err(
                "DJ Link machine authority is blocked: stale_machine_transaction".to_string(),
            );
        }
        Ok(&self.settings)
    }

    /// Restores the non-secret machine state plus its credential generation
    /// from a resolved local-data directory. A V2 active journal is recovered
    /// to its last accepted authority only when its separate Credential
    /// Manager rollback record exactly proves that authority.
    pub fn initialize(&mut self, local_data_dir: &Path) {
        let path = dj_link_machine_settings_path(local_data_dir);
        self.settings_path = Some(path.clone());
        self.restored_credential = false;
        self.wire_token = None;
        self.credential_cleanup_pending = false;
        self.block_reason = None;
        let settings = match load_dj_link_machine_settings_from_path(&path) {
            DjLinkMachineSettingsLoadOutcome::MissingDefaults(settings) => {
                self.settings = settings;
                self.block_reason = Some("missing_machine_settings".to_string());
                return;
            }
            DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { .. } => {
                self.settings = DjLinkMachineSettingsV2::default();
                self.block_reason = Some("corrupt_machine_settings".to_string());
                return;
            }
            DjLinkMachineSettingsLoadOutcome::BlockedFutureVersion { .. } => {
                self.settings = DjLinkMachineSettingsV2::default();
                self.block_reason = Some("future_machine_settings_version".to_string());
                return;
            }
            DjLinkMachineSettingsLoadOutcome::BlockedRetiredVersion { .. } => {
                self.settings = DjLinkMachineSettingsV2::default();
                self.block_reason = Some("retired_machine_settings_version".to_string());
                return;
            }
            DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        };
        let store = match platform_dj_link_credential_store() {
            Ok(store) => store,
            Err(_) => {
                self.initialize_without_credential_store(settings);
                return;
            }
        };
        let persistence = PathDjLinkSettingsPersistence { path };
        self.initialize_with_dependencies(settings, &persistence, store.as_ref());
    }

    /// Preserve a durable cleanup instruction even when the local credential
    /// provider cannot be opened. A later supported launch must retry the
    /// same marker before restoring any authority; reporting a generic
    /// platform error here would hide that operator action.
    fn initialize_without_credential_store(&mut self, settings: DjLinkMachineSettingsV2) {
        self.settings = settings;
        self.wire_token = None;
        self.restored_credential = false;
        self.credential_cleanup_pending = self.settings.disarm_cleanup_pending;
        self.block_reason = if self.settings.disarm_cleanup_pending {
            Some("disarm_cleanup_pending".to_string())
        } else {
            self.settings
                .credential_generation
                .is_some()
                .then(|| "credential_platform_unsupported".to_string())
        };
    }

    /// Returns a closed-surface DJ-only config. Generic HTTP/Web Remote is
    /// deliberately absent from an automatic start.
    pub fn dj_only_config(&self) -> Result<RemoteControlConfig, String> {
        let settings = self.ready_settings()?;
        if !settings.auto_start_armed || !self.restored_credential {
            return Err("DJ Link machine authority is not explicitly armed".to_string());
        }
        let bind_ip = settings
            .bind_ip
            .clone()
            .ok_or_else(|| "DJ Link machine binding is missing an IP".to_string())?;
        let port = settings
            .bind_port
            .ok_or_else(|| "DJ Link machine binding is missing a port".to_string())?;
        let token = self
            .wire_token
            .clone()
            .ok_or_else(|| "DJ Link machine credential is missing".to_string())?;
        Ok(RemoteControlConfig {
            bind_ip: bind_ip.clone(),
            port,
            pairing_pin: String::new(),
            allow_lan: true,
            web_remote_enabled: false,
            dj_link_enabled: true,
            dj_link_bind_ip: Some(bind_ip),
            dj_link_token: Some(token),
            ..RemoteControlConfig::default()
        })
    }

    /// The listener always applies the production policy. Tests can inject a
    /// deterministic fresh-binding observer, but never bypass the tuple,
    /// armed, restored-credential, or token checks themselves.
    fn validate_listener_config_with(
        &mut self,
        config: &RemoteControlConfig,
        validate_binding: &dyn Fn(&str, &str, &str) -> Result<(), String>,
    ) -> Result<(), String> {
        if !config.dj_link_enabled {
            // A renderer (or another IPC caller) cannot replace an explicitly
            // armed machine authority with a Web-only listener. The machine
            // state remains valid and armed; this is a request mismatch, not
            // a reason to poison that authority's restored credential.
            if self.settings.auto_start_armed {
                return Err(
                    "DJ Link machine authority is armed; start an exact DJ Link listener or disarm it first"
                        .to_string(),
                );
            }
            return Ok(());
        }
        let settings = self.ready_settings()?.clone();
        if !settings.auto_start_armed || !self.restored_credential {
            self.block_reason = Some("credential_missing_or_unarmed".to_string());
            return Err(
                "DJ Link machine authority is blocked: credential_missing_or_unarmed".to_string(),
            );
        }
        let bind_ip = settings
            .bind_ip
            .as_deref()
            .ok_or_else(|| "DJ Link machine authority is blocked: binding_missing".to_string())?;
        let network_guid = settings
            .network_guid
            .as_deref()
            .ok_or_else(|| "DJ Link machine authority is blocked: binding_missing".to_string())?;
        let adapter_guid = settings
            .adapter_guid
            .as_deref()
            .ok_or_else(|| "DJ Link machine authority is blocked: binding_missing".to_string())?;
        let bind_port = settings
            .bind_port
            .ok_or_else(|| "DJ Link machine authority is blocked: binding_missing".to_string())?;
        if config.bind_ip.trim() != bind_ip
            || config.dj_link_bind_ip.as_deref().map(str::trim) != Some(bind_ip)
            || config.port != bind_port
            || !config.allow_lan
        {
            self.block_reason = Some("listener_config_mismatch".to_string());
            return Err(
                "DJ Link machine authority is blocked: listener_config_mismatch".to_string(),
            );
        }
        if self.wire_token.is_none() {
            self.block_reason = Some("credential_missing".to_string());
            return Err("DJ Link machine authority is blocked: credential_missing".to_string());
        }
        if let Err(reason) = validate_binding(network_guid, adapter_guid, bind_ip) {
            self.block_reason = Some(reason.clone());
            return Err(format!("DJ Link machine authority is blocked: {reason}"));
        }
        Ok(())
    }

    pub fn prepare_listener_config(
        &mut self,
        config: &mut RemoteControlConfig,
    ) -> Result<(), String> {
        self.prepare_listener_config_with(config, &validate_network_binding)
    }

    fn prepare_listener_config_with(
        &mut self,
        config: &mut RemoteControlConfig,
        validate_binding: &dyn Fn(&str, &str, &str) -> Result<(), String>,
    ) -> Result<(), String> {
        self.validate_listener_config_with(config, validate_binding)?;
        if config.dj_link_enabled {
            config.dj_link_token = Some(
                self.wire_token
                    .clone()
                    .ok_or_else(|| "DJ Link machine credential is missing".to_string())?,
            );
        }
        Ok(())
    }

    /// Atomically journals an explicit arm/rotation. `request=None` retains
    /// the stored complete tuple but still revalidates it before rotating.
    pub fn arm_or_rotate(
        &mut self,
        request: Option<DjLinkMachineArmRequest>,
    ) -> Result<String, String> {
        let path = self
            .settings_path
            .clone()
            .ok_or_else(|| "DJ Link machine settings are not initialized".to_string())?;
        let store = platform_dj_link_credential_store().map_err(|error| error.to_string())?;
        let persistence = PathDjLinkSettingsPersistence { path };
        self.arm_or_rotate_with_dependencies(
            request,
            &persistence,
            store.as_ref(),
            &OsDjLinkTokenGenerator,
            &validate_network_binding,
        )
    }

    /// Disarming writes `armed=false` before revocation so a later crash never
    /// auto-starts. The secret is then revoked and the generation cleared.
    pub fn disarm(&mut self) -> Result<(), String> {
        let path = self
            .settings_path
            .clone()
            .ok_or_else(|| "DJ Link machine settings are not initialized".to_string())?;
        let store = platform_dj_link_credential_store().map_err(|error| error.to_string())?;
        let persistence = PathDjLinkSettingsPersistence { path };
        self.disarm_with_dependencies(&persistence, store.as_ref())
    }

    fn disarm_with_dependencies(
        &mut self,
        persistence: &dyn DjLinkSettingsPersistence,
        store: &dyn DjLinkCredentialStore,
    ) -> Result<(), String> {
        if matches!(
            self.block_reason.as_deref(),
            Some(
                "corrupt_machine_settings"
                    | "future_machine_settings_version"
                    | "retired_machine_settings_version"
            )
        ) {
            return Err(
                "DJ Link machine authority is blocked: protected_machine_settings".to_string(),
            );
        }
        if self.settings.transaction_state != DjLinkMachineTransactionState::Idle {
            self.recover_active_transaction(persistence, store)?;
        }
        if self.settings.disarm_cleanup_pending {
            return self.complete_disarm_cleanup(persistence, store);
        }
        // Record cleanup intent before the first credential mutation. A
        // restart must never mistake a partially revoked disarm for a valid
        // restored gN authority.
        let cleanup_intent = build_disarmed_settings(&self.settings, false, true)?;
        if let Err(error) = persistence.persist(&cleanup_intent) {
            // A settings writer can report failure after an atomic rename. Do
            // not leave this process armed merely because the error cannot
            // prove whether the marker reached disk. The next initialization
            // will read the durable truth and either complete cleanup or keep
            // the machine explicitly blocked.
            self.settings = cleanup_intent;
            self.wire_token = None;
            self.restored_credential = false;
            self.credential_cleanup_pending = true;
            self.block_reason = Some("disarm_cleanup_pending".to_string());
            return Err(error);
        }
        self.settings = cleanup_intent;
        self.complete_disarm_cleanup(persistence, store)
    }

    /// A durable disarm marker is an instruction to remove both possible
    /// credential slots before this process may restore or inject anything.
    /// Attempt both deletions on every pass: a failed primary delete must not
    /// leave a stale rollback slot indefinitely, and vice versa.
    fn complete_disarm_cleanup(
        &mut self,
        persistence: &dyn DjLinkSettingsPersistence,
        store: &dyn DjLinkCredentialStore,
    ) -> Result<(), String> {
        self.wire_token = None;
        self.restored_credential = false;
        let primary_error = store.revoke_token().err();
        let rollback_error = store.revoke_rollback_token().err();
        if primary_error.is_some() || rollback_error.is_some() {
            self.credential_cleanup_pending = true;
            self.block_reason = Some("disarm_cleanup_pending".to_string());
            let primary = primary_error
                .map(|error| format!("primary: {error}"))
                .unwrap_or_else(|| "primary: removed".to_string());
            let rollback = rollback_error
                .map(|error| format!("rollback: {error}"))
                .unwrap_or_else(|| "rollback: removed".to_string());
            return Err(format!(
                "DJ Link credential cleanup is pending ({primary}; {rollback})"
            ));
        }
        self.settings = match persist_disarmed_settings(persistence, &self.settings, true, false) {
            Ok(settings) => settings,
            Err(error) => {
                self.credential_cleanup_pending = true;
                self.block_reason = Some("disarm_cleanup_pending".to_string());
                return Err(error);
            }
        };
        self.credential_cleanup_pending = false;
        self.block_reason = None;
        Ok(())
    }

    fn initialize_with_dependencies(
        &mut self,
        settings: DjLinkMachineSettingsV2,
        persistence: &dyn DjLinkSettingsPersistence,
        credentials: &dyn DjLinkCredentialStore,
    ) {
        self.settings = settings;
        if self.settings.disarm_cleanup_pending {
            if let Err(reason) = self.complete_disarm_cleanup(persistence, credentials) {
                self.wire_token = None;
                self.restored_credential = false;
                self.credential_cleanup_pending = true;
                let _ = reason;
                self.block_reason = Some("disarm_cleanup_pending".to_string());
                return;
            }
        }
        if self.settings.transaction_state != DjLinkMachineTransactionState::Idle {
            if let Err(reason) = self.recover_active_transaction(persistence, credentials) {
                self.block_reason = Some(reason);
                return;
            }
        }
        self.restore_idle_credential(credentials);
    }

    fn restore_idle_credential(&mut self, credentials: &dyn DjLinkCredentialStore) {
        debug_assert_eq!(
            self.settings.transaction_state,
            DjLinkMachineTransactionState::Idle
        );
        // Initialization may follow a rollback or an in-process retry. Clear
        // the process-only injection state before every credential read so a
        // missing/read-failed idle record can never inherit a prior accepted
        // authority and claim `credential_ready`.
        self.wire_token = None;
        self.restored_credential = false;
        let Some(expected_generation) = self.settings.credential_generation else {
            self.block_reason = self
                .settings
                .auto_start_armed
                .then(|| "credential_missing".to_string());
            return;
        };
        let record = match credentials.load_token() {
            Ok(Some(record)) if record.generation == expected_generation => record,
            Ok(Some(_)) => {
                self.block_reason = Some("credential_generation_mismatch".to_string());
                return;
            }
            Ok(None) => {
                self.block_reason = Some("credential_missing".to_string());
                return;
            }
            Err(_) => {
                self.block_reason = Some("credential_read_failed".to_string());
                return;
            }
        };
        // An orphaned rollback credential can only be removed after the
        // primary record and idle settings have both verified. Cleanup is
        // deliberately nonfatal: blocking here would turn a successful
        // commit into a permanent no-show state. Initialization retries it.
        self.credential_cleanup_pending = credentials.revoke_rollback_token().is_err();
        self.wire_token = Some(wire_token(&record.token));
        self.restored_credential = true;
        self.block_reason = None;
    }

    /// A V2 journal never promotes an unfinished rotation. Every non-idle
    /// state is rolled back to the last accepted settings and credential when
    /// that exact preimage is available; all other combinations remain
    /// blocked rather than guessing which token a remote DJ computer saw.
    fn recover_active_transaction(
        &mut self,
        persistence: &dyn DjLinkSettingsPersistence,
        credentials: &dyn DjLinkCredentialStore,
    ) -> Result<(), String> {
        let active = self.settings.clone();
        let rollback = active
            .rollback
            .clone()
            .ok_or_else(|| "stale_machine_transaction".to_string())?;
        let old_generation = rollback.credential_generation;
        let new_generation = active
            .credential_generation
            .ok_or_else(|| "stale_machine_transaction".to_string())?;
        let primary = credentials
            .load_token()
            .map_err(|_| "credential_read_failed".to_string())?;
        let rollback_record = credentials
            .load_rollback_token()
            .map_err(|_| "rollback_credential_read_failed".to_string())?;

        match old_generation {
            Some(old_generation) => {
                let rollback_record = rollback_record
                    .filter(|record| record.generation == old_generation)
                    .ok_or_else(|| "stale_machine_transaction".to_string())?;
                match primary.as_ref().map(|record| record.generation) {
                    Some(generation)
                        if generation == old_generation || generation == new_generation => {}
                    None => {}
                    Some(_) => return Err("stale_machine_transaction".to_string()),
                }
                credentials
                    .save_token(old_generation, &rollback_record.token)
                    .map_err(|_| "credential_rollback_restore_failed".to_string())?;
            }
            None => {
                if rollback_record.is_some() {
                    return Err("stale_machine_transaction".to_string());
                }
                match primary.as_ref().map(|record| record.generation) {
                    None => {}
                    Some(generation) if generation == new_generation => credentials
                        .revoke_token()
                        .map_err(|_| "credential_rollback_revoke_failed".to_string())?,
                    Some(_) => return Err("stale_machine_transaction".to_string()),
                }
            }
        }

        let restored = active
            .restore_rollback(active.revision)
            .map_err(|_| "stale_machine_transaction".to_string())?;
        persistence
            .persist(&restored)
            .map_err(|_| "machine_settings_rollback_persist_failed".to_string())?;
        self.settings = restored;
        self.credential_cleanup_pending = credentials.revoke_rollback_token().is_err();
        self.wire_token = None;
        self.restored_credential = false;
        self.block_reason = None;
        Ok(())
    }

    fn arm_or_rotate_with_dependencies(
        &mut self,
        request: Option<DjLinkMachineArmRequest>,
        persistence: &dyn DjLinkSettingsPersistence,
        credentials: &dyn DjLinkCredentialStore,
        token_generator: &dyn DjLinkTokenGenerator,
        validate_binding: &dyn Fn(&str, &str, &str) -> Result<(), String>,
    ) -> Result<String, String> {
        if self.settings.transaction_state != DjLinkMachineTransactionState::Idle {
            self.recover_active_transaction(persistence, credentials)
                .map_err(|reason| format!("DJ Link machine authority is blocked: {reason}"))?;
        }
        if let Some(reason) = self.block_reason.as_deref() {
            if reason != "missing_machine_settings" && reason != "credential_missing" {
                return Err(format!("DJ Link machine authority is blocked: {reason}"));
            }
        }

        let old_settings = self
            .settings
            .clone()
            .validated()
            .map_err(|error| format!("DJ Link machine settings are invalid: {error}"))?;
        let old_record = match old_settings.credential_generation {
            Some(expected_generation) => match credentials.load_token() {
                Ok(Some(record)) if record.generation == expected_generation => Some(record),
                Ok(Some(_)) => {
                    return Err("DJ Link credential generation is mismatched".to_string())
                }
                Ok(None) => None,
                Err(_) => return Err("DJ Link credential could not be read".to_string()),
            },
            None => match credentials.load_token() {
                Ok(None) => None,
                Ok(Some(_)) => {
                    return Err("DJ Link credential generation is mismatched".to_string())
                }
                Err(_) => return Err("DJ Link credential could not be read".to_string()),
            },
        };
        // The persisted generation is a monotonic namespace even if the
        // matching primary credential has disappeared. Roll back to an
        // unarmed/no-generation preimage, but allocate the next token from
        // the last persisted generation rather than ever reusing g1.
        let next_generation_from = old_settings
            .credential_generation_high_water
            .max(old_settings.credential_generation.unwrap_or(0));
        let mut rollback_settings = old_settings.clone();
        if old_settings.credential_generation.is_some() && old_record.is_none() {
            rollback_settings.credential_generation = None;
            rollback_settings.auto_start_armed = false;
        }
        let rollback = DjLinkMachineRollback::from_stable(&rollback_settings)
            .map_err(|error| format!("DJ Link rollback settings are invalid: {error}"))?;

        let mut desired = old_settings;
        if let Some(request) = request {
            if request.bind_port == 0 {
                return Err("DJ Link bind port must be between 1 and 65535".to_string());
            }
            validate_binding(
                &request.network_guid,
                &request.adapter_guid,
                &request.bind_ip,
            )?;
            desired.network_guid = Some(request.network_guid);
            desired.adapter_guid = Some(request.adapter_guid);
            desired.bind_ip = Some(request.bind_ip);
            desired.bind_port = Some(request.bind_port);
        } else {
            let network = desired
                .network_guid
                .as_deref()
                .ok_or_else(|| "DJ Link is not bound to a network".to_string())?;
            let adapter = desired
                .adapter_guid
                .as_deref()
                .ok_or_else(|| "DJ Link is not bound to an adapter".to_string())?;
            let bind_ip = desired
                .bind_ip
                .as_deref()
                .ok_or_else(|| "DJ Link has no persisted bind endpoint".to_string())?;
            if desired.bind_port.unwrap_or_default() == 0 {
                return Err("DJ Link has no persisted bind endpoint".to_string());
            }
            validate_binding(network, adapter, bind_ip)?;
        }
        // Generate before writing either durable authority. A CSPRNG failure
        // therefore has no rollback or recovery side effect.
        let token = token_generator.generate()?;
        let generation = next_generation_from
            .checked_add(1)
            .filter(|generation| *generation != 0)
            .ok_or_else(|| "DJ Link credential generation is exhausted".to_string())?;
        desired.credential_generation = rollback_settings.credential_generation;
        desired.auto_start_armed = true;
        desired.disarm_cleanup_pending = false;
        desired.transaction_state = DjLinkMachineTransactionState::Idle;
        desired.rollback = None;
        desired = desired
            .validated()
            .map_err(|error| format!("DJ Link machine settings are invalid: {error}"))?;
        let prepared = desired
            .begin_prepare_commit_with_rollback(self.settings.revision, generation, rollback)
            .map_err(|error| format!("DJ Link machine journal could not begin: {error}"))?;
        // Clear any orphan only while the old stable primary is still
        // authoritative, then durable-copy the old token *before* publishing
        // the active journal. A crash after this point always has its old
        // credential and non-secret preimage available for recovery.
        credentials
            .revoke_rollback_token()
            .map_err(|error| error.to_string())?;
        if let Some(old_record) = old_record.as_ref() {
            credentials
                .save_rollback_token(old_record.generation, &old_record.token)
                .map_err(|error| error.to_string())?;
        }
        self.settings = prepared.clone();
        self.wire_token = None;
        self.restored_credential = false;
        self.block_reason = Some("credential_commit_incomplete".to_string());
        if let Err(error) = persistence.persist(&prepared) {
            return self.rollback_after_failed_arm(persistence, credentials, error);
        }
        let committing = prepared
            .stage_committing(prepared.revision)
            .map_err(|error| format!("DJ Link machine journal could not commit: {error}"));
        let committing = match committing {
            Ok(committing) => committing,
            Err(error) => return self.rollback_after_failed_arm(persistence, credentials, error),
        };
        self.settings = committing.clone();
        if let Err(error) = persistence.persist(&committing) {
            return self.rollback_after_failed_arm(persistence, credentials, error);
        }
        if let Err(error) = credentials.save_token(generation, &token) {
            return self.rollback_after_failed_arm(persistence, credentials, error.to_string());
        }
        let stable = committing
            .finish_commit(committing.revision)
            .map_err(|error| format!("DJ Link machine journal could not finish: {error}"));
        let stable = match stable {
            Ok(stable) => stable,
            Err(error) => return self.rollback_after_failed_arm(persistence, credentials, error),
        };
        if let Err(error) = persistence.persist(&stable) {
            // Keep the active state in memory while compensating even if the
            // writer returned an error after a rename. The recovery write
            // deliberately re-establishes the old authority either way.
            self.settings = committing;
            return self.rollback_after_failed_arm(persistence, credentials, error);
        }
        let wire = wire_token(&token);
        self.settings = stable;
        self.wire_token = Some(wire.clone());
        self.block_reason = None;
        self.restored_credential = true;
        // The show-once return must have no fallible work after the stable
        // settings commit. Retain an explicit, nonblocking cleanup-pending
        // signal and retry the rollback deletion at the next initialization.
        self.credential_cleanup_pending = credentials.revoke_rollback_token().is_err();
        Ok(wire)
    }

    fn rollback_after_failed_arm(
        &mut self,
        persistence: &dyn DjLinkSettingsPersistence,
        credentials: &dyn DjLinkCredentialStore,
        original_error: String,
    ) -> Result<String, String> {
        match self.recover_active_transaction(persistence, credentials) {
            Ok(()) => {
                // Compensation restores both durable g1 and this process's
                // injection authority. Returning an arm error must not leave
                // the old accepted token silently unusable until restart.
                self.restore_idle_credential(credentials);
                match self.block_reason.as_deref() {
                    None => Err(original_error),
                    Some(reason) => Err(format!(
                        "{original_error}; DJ Link recovery blocked: {reason}"
                    )),
                }
            }
            Err(recovery_error) => {
                self.block_reason = Some(recovery_error.clone());
                Err(format!(
                    "{original_error}; DJ Link recovery blocked: {recovery_error}"
                ))
            }
        }
    }

    #[cfg(test)]
    pub fn install_test_wire_token(&mut self, token: &str) {
        self.wire_token = Some(token.to_string());
    }

    /// Test-only setup still uses the production listener policy. It supplies
    /// a complete in-memory authority and injects only the fresh NLM observer
    /// because unit fixtures do not own a real Windows network binding.
    #[cfg(test)]
    pub fn install_test_armed_listener_authority(&mut self, bind_ip: &str, port: u16, token: &str) {
        self.settings = DjLinkMachineSettingsV2 {
            revision: 1,
            transaction_state: DjLinkMachineTransactionState::Idle,
            credential_generation: Some(1),
            credential_generation_high_water: 1,
            adapter_guid: Some("11111111-1111-1111-1111-111111111111".to_string()),
            network_guid: Some("22222222-2222-2222-2222-222222222222".to_string()),
            bind_ip: Some(bind_ip.to_string()),
            bind_port: Some(port),
            auto_start_armed: true,
            rollback: None,
            ..DjLinkMachineSettingsV2::default()
        }
        .validated()
        .expect("test DJ Link authority must be valid");
        self.block_reason = None;
        self.restored_credential = true;
        self.credential_cleanup_pending = false;
        self.wire_token = Some(token.to_string());
    }

    /// Models the durable post-disarm authority for listener lifecycle tests.
    /// Credential deletion/retry semantics are separately exercised through
    /// the production-shaped persistence dependency tests.
    #[cfg(test)]
    pub fn install_test_disarmed_listener_authority(&mut self) {
        self.settings = DjLinkMachineSettingsV2::default();
        self.block_reason = None;
        self.restored_credential = false;
        self.credential_cleanup_pending = false;
        self.wire_token = None;
    }

    #[cfg(test)]
    pub fn prepare_listener_config_with_test_binding(
        &mut self,
        config: &mut RemoteControlConfig,
    ) -> Result<(), String> {
        // The test observer is deliberately exact rather than an unconditional
        // success: production tuple/armed/token policy remains active and the
        // injected NLM result accepts only this authority's full tuple.
        let expected_network = self.settings.network_guid.clone();
        let expected_adapter = self.settings.adapter_guid.clone();
        let expected_ip = self.settings.bind_ip.clone();
        self.prepare_listener_config_with(config, &move |network, adapter, ip| {
            (expected_network.as_deref() == Some(network)
                && expected_adapter.as_deref() == Some(adapter)
                && expected_ip.as_deref() == Some(ip))
            .then_some(())
            .ok_or_else(|| "test_network_binding_mismatch".to_string())
        })
    }
}

fn wire_token(token: &DjLinkToken) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token.expose())
}

fn machine_settings_next_revision(settings: &DjLinkMachineSettingsV2) -> Result<u64, String> {
    settings
        .revision
        .checked_add(1)
        .filter(|revision| *revision != u64::MAX)
        .ok_or_else(|| "DJ Link machine settings revision is exhausted".to_string())
}

fn persist_disarmed_settings(
    persistence: &dyn DjLinkSettingsPersistence,
    settings: &DjLinkMachineSettingsV2,
    clear_generation: bool,
    disarm_cleanup_pending: bool,
) -> Result<DjLinkMachineSettingsV2, String> {
    let next = build_disarmed_settings(settings, clear_generation, disarm_cleanup_pending)?;
    persistence.persist(&next)?;
    Ok(next)
}

fn build_disarmed_settings(
    settings: &DjLinkMachineSettingsV2,
    clear_generation: bool,
    disarm_cleanup_pending: bool,
) -> Result<DjLinkMachineSettingsV2, String> {
    let mut next = settings.clone();
    next.revision = machine_settings_next_revision(settings)?;
    next.transaction_state = DjLinkMachineTransactionState::Idle;
    next.auto_start_armed = false;
    next.disarm_cleanup_pending = disarm_cleanup_pending;
    next.rollback = None;
    if clear_generation {
        next.credential_generation = None;
    }
    next = next
        .validated()
        .map_err(|error| format!("DJ Link machine settings are invalid: {error}"))?;
    Ok(next)
}

fn trust_block_reason(decision: DjLinkTrustDecision) -> Result<(), String> {
    match decision {
        DjLinkTrustDecision::Allow => Ok(()),
        DjLinkTrustDecision::Block(reason) => Err(match reason {
            dj_link_network::DjLinkTrustBlockReason::StoredIdentityInvalid => {
                "trust_stored_identity_invalid"
            }
            dj_link_network::DjLinkTrustBlockReason::PassesDisagree => "trust_passes_disagree",
            dj_link_network::DjLinkTrustBlockReason::DuplicateNetworkAmbiguity => {
                "trust_duplicate_network_ambiguity"
            }
            dj_link_network::DjLinkTrustBlockReason::StoredNetworkAbsent => "trust_network_absent",
            dj_link_network::DjLinkTrustBlockReason::MultipleStoredNetworkMatches => {
                "trust_multiple_network_matches"
            }
            dj_link_network::DjLinkTrustBlockReason::StoredAdapterAbsent => "trust_adapter_absent",
            dj_link_network::DjLinkTrustBlockReason::ObserverFailed(_) => "observer_failed",
        }
        .to_string()),
    }
}

#[cfg(target_os = "windows")]
pub fn discover_wired_candidates() -> Result<Vec<DjLinkWiredCandidate>, String> {
    let observer =
        dj_link_network::NlmTrustObserver::spawn().map_err(|_| "observer_failed".to_string())?;
    let report = observer
        .discover_ipv4_candidates()
        .map_err(|failure| format!("wired_candidate_discovery_failed:{failure:?}"))?;
    observer
        .shutdown()
        .map_err(|_| "observer_shutdown_failed".to_string())?;
    Ok(report
        .eligible()
        .iter()
        .map(|candidate| DjLinkWiredCandidate {
            network_guid: candidate.binding().network_guid().to_string(),
            adapter_guid: candidate.binding().adapter_guid().to_string(),
            bind_ip: candidate.binding().bind_ipv4().to_string(),
            adapter_alias: candidate.adapter_alias().map(str::to_string),
        })
        .collect())
}

#[cfg(not(target_os = "windows"))]
pub fn discover_wired_candidates() -> Result<Vec<DjLinkWiredCandidate>, String> {
    Err("machine_local_platform_unsupported".to_string())
}

#[cfg(target_os = "windows")]
fn validate_network_binding(
    network_guid: &str,
    adapter_guid: &str,
    bind_ip: &str,
) -> Result<(), String> {
    let bind_ipv4 = bind_ip
        .parse()
        .map_err(|_| "binding_ip_invalid".to_string())?;
    let binding = DjLinkIpv4BindingKey::new(
        network_guid.to_string(),
        adapter_guid.to_string(),
        bind_ipv4,
    )
    .map_err(|_| "binding_identity_invalid".to_string())?;
    let observer =
        dj_link_network::NlmTrustObserver::spawn().map_err(|_| "observer_failed".to_string())?;
    let validation = (|| {
        let trust = observer
            .evaluate_with_dwell(network_guid, adapter_guid, DJ_LINK_NETWORK_SETTLE_DWELL)
            .map_err(|_| "observer_failed".to_string())?;
        trust_block_reason(trust)?;
        observer
            .validate_ipv4_binding(&binding)
            .map_err(|_| "binding_tuple_stale_or_untrusted".to_string())?;
        Ok(())
    })();
    observer
        .shutdown()
        .map_err(|_| "observer_shutdown_failed".to_string())?;
    validation
}

#[cfg(not(target_os = "windows"))]
fn validate_network_binding(
    _network_guid: &str,
    _adapter_guid: &str,
    _bind_ip: &str,
) -> Result<(), String> {
    Err("machine_local_platform_unsupported".to_string())
}

#[cfg(test)]
#[path = "tests/dj_link_persistence_runtime_tests.rs"]
mod tests;
