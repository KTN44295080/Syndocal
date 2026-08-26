//! Strict V1 renderer-to-native requests for the live-audio commands.
//!
//! This module deliberately owns the JSON boundary only. The runtime keeps
//! its established snake_case models, while these DTOs accept the one V1
//! camelCase wire representation and are converted at the command boundary.

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{Map, Value};
use tauri::{
    ipc::{CommandArg, CommandItem, InvokeBody, InvokeError},
    Runtime,
};

pub(crate) const LIVE_AUDIO_IPC_SCHEMA_VERSION: u16 = 1;

/// A nullable value that must still be present on the wire.
///
/// `Option<T>` alone cannot represent the required wire-presence contract.
/// The strict request decoder checks each required nullable key before Serde
/// decodes this wrapper, so an omitted field cannot be treated as `null`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequiredNullable<T>(Option<T>);

impl<T> RequiredNullable<T> {
    pub(crate) fn into_option(self) -> Option<T> {
        self.0
    }
}

impl<'de, T> Deserialize<'de> for RequiredNullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Self)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LiveAudioInputBackendV1 {
    WasapiShared,
    Asio,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum LiveAudioChannelMixV1 {
    AverageAll,
    Single {
        channel_index: u16,
    },
    StereoPair {
        left_channel_index: u16,
        right_channel_index: u16,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ListAudioInputDevicesRequestV1 {
    schema_version: u16,
    pub(crate) backend: LiveAudioInputBackendV1,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LiveAudioInputCapabilitiesRequestV1 {
    schema_version: u16,
    pub(crate) backend: LiveAudioInputBackendV1,
    pub(crate) device_id: RequiredNullable<String>,
    pub(crate) sample_rate: RequiredNullable<u32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartLiveAudioInputRequestV1 {
    schema_version: u16,
    pub(crate) backend: LiveAudioInputBackendV1,
    pub(crate) device_id: RequiredNullable<String>,
    pub(crate) sample_rate: RequiredNullable<u32>,
    pub(crate) stream_channels: RequiredNullable<u16>,
    pub(crate) sample_format: RequiredNullable<String>,
    pub(crate) buffer_frames: RequiredNullable<u32>,
    pub(crate) channel_mix: LiveAudioChannelMixV1,
}

pub(crate) trait StrictLiveAudioRequestV1: DeserializeOwned {
    fn schema_version(&self) -> u16;

    fn required_nullable_fields() -> &'static [&'static str];

    fn validate_before_deserialize(_request: &Map<String, Value>) -> Result<(), String> {
        Ok(())
    }
}

impl StrictLiveAudioRequestV1 for ListAudioInputDevicesRequestV1 {
    fn schema_version(&self) -> u16 {
        self.schema_version
    }

    fn required_nullable_fields() -> &'static [&'static str] {
        &[]
    }
}

impl StrictLiveAudioRequestV1 for LiveAudioInputCapabilitiesRequestV1 {
    fn schema_version(&self) -> u16 {
        self.schema_version
    }

    fn required_nullable_fields() -> &'static [&'static str] {
        &["deviceId", "sampleRate"]
    }
}

impl StrictLiveAudioRequestV1 for StartLiveAudioInputRequestV1 {
    fn schema_version(&self) -> u16 {
        self.schema_version
    }

    fn required_nullable_fields() -> &'static [&'static str] {
        &[
            "deviceId",
            "sampleRate",
            "streamChannels",
            "sampleFormat",
            "bufferFrames",
        ]
    }

    fn validate_before_deserialize(request: &Map<String, Value>) -> Result<(), String> {
        let channel_mix = request
            .get("channelMix")
            .ok_or_else(|| "live audio IPC V1 requires channelMix".to_string())?;
        let channel_mix = channel_mix
            .as_object()
            .ok_or_else(|| "live audio IPC V1 channelMix must be an object".to_string())?;
        let mode = channel_mix
            .get("mode")
            .and_then(Value::as_str)
            .ok_or_else(|| "live audio IPC V1 channelMix requires string mode".to_string())?;
        let required_fields: &[&str] = match mode {
            "averageAll" => &["mode"],
            "single" => &["mode", "channelIndex"],
            "stereoPair" => &["mode", "leftChannelIndex", "rightChannelIndex"],
            _ => {
                return Err(format!(
                    "live audio IPC V1 channelMix mode {mode:?} is unsupported"
                ))
            }
        };
        if channel_mix.len() != required_fields.len()
            || required_fields
                .iter()
                .any(|field| !channel_mix.contains_key(*field))
        {
            return Err(format!(
                "live audio IPC V1 channelMix mode {mode:?} has an invalid field set"
            ));
        }
        Ok(())
    }
}

/// The only accepted Tauri argument shape for the V1 live-audio commands.
///
/// Tauri normally extracts one named argument and consequently ignores other
/// top-level keys. This adapter instead validates the entire invoke body so
/// every command accepts exactly `{ request: { ... } }`.
#[derive(Debug)]
pub(crate) struct StrictLiveAudioRequest<T>(T);

impl<T> StrictLiveAudioRequest<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T> StrictLiveAudioRequest<T>
where
    T: StrictLiveAudioRequestV1,
{
    fn from_body(command_name: &str, body: &InvokeBody) -> Result<Self, String> {
        let InvokeBody::Json(value) = body else {
            return Err(format!(
                "invalid args `request` for command `{command_name}`: live audio IPC V1 requires a JSON object containing exactly request"
            ));
        };
        let Some(outer) = value.as_object() else {
            return Err(format!(
                "invalid args `request` for command `{command_name}`: live audio IPC V1 requires exactly one outer request object"
            ));
        };
        if outer.len() != 1 || !outer.contains_key("request") {
            return Err(format!(
                "invalid args `request` for command `{command_name}`: live audio IPC V1 requires exactly one outer request object"
            ));
        }
        let request_value = outer
            .get("request")
            .expect("request key was checked before access");
        let Some(request_object) = request_value.as_object() else {
            return Err(format!(
                "invalid args `request` for command `{command_name}`: live audio IPC V1 request must be an object"
            ));
        };
        for field in T::required_nullable_fields() {
            if !request_object.contains_key(*field) {
                return Err(format!(
                    "invalid args `request` for command `{command_name}`: live audio IPC V1 requires nullable field {field} to be present (use null when unset)"
                ));
            }
        }
        T::validate_before_deserialize(request_object).map_err(|error| {
            format!("invalid args `request` for command `{command_name}`: {error}")
        })?;
        let request = serde_json::from_value::<T>(request_value.clone()).map_err(|error| {
            format!("invalid args `request` for command `{command_name}`: {error}")
        })?;
        if request.schema_version() != LIVE_AUDIO_IPC_SCHEMA_VERSION {
            return Err(format!(
                "invalid args `request` for command `{command_name}`: live audio IPC schemaVersion must be {LIVE_AUDIO_IPC_SCHEMA_VERSION}"
            ));
        }
        Ok(Self(request))
    }
}

/// Admits the raw invoke body for the three V1 live-audio commands before the
/// application's runtime-dispatch preflight reaches managed state or locks.
///
/// The generated command handler retains `StrictLiveAudioRequest` as its
/// defense in depth, but its argument extraction occurs too late to protect
/// the dispatch fence for `start_live_audio_input`.
pub(crate) fn admit_live_audio_ipc_v1_invoke_body(
    command_name: &str,
    body: &InvokeBody,
) -> Result<(), String> {
    match command_name {
        "list_audio_input_devices" => {
            StrictLiveAudioRequest::<ListAudioInputDevicesRequestV1>::from_body(
                command_name,
                body,
            )?;
        }
        "get_live_audio_input_capabilities" => {
            StrictLiveAudioRequest::<LiveAudioInputCapabilitiesRequestV1>::from_body(
                command_name,
                body,
            )?;
        }
        "start_live_audio_input" => {
            StrictLiveAudioRequest::<StartLiveAudioInputRequestV1>::from_body(command_name, body)?;
        }
        _ => {}
    }
    Ok(())
}

impl<'de, R, T> CommandArg<'de, R> for StrictLiveAudioRequest<T>
where
    R: Runtime,
    T: StrictLiveAudioRequestV1,
{
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        Self::from_body(command.name, command.message.payload())
            .map_err(|error| InvokeError(Value::String(error)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse<T>(command_name: &str, value: Value) -> Result<StrictLiveAudioRequest<T>, String>
    where
        T: StrictLiveAudioRequestV1,
    {
        StrictLiveAudioRequest::<T>::from_body(command_name, &InvokeBody::Json(value))
    }

    #[test]
    fn v1_accepts_exact_outer_shape_and_camel_case_requests() {
        let list = parse::<ListAudioInputDevicesRequestV1>(
            "list_audio_input_devices",
            json!({ "request": { "schemaVersion": 1, "backend": "wasapiShared" } }),
        )
        .unwrap()
        .into_inner();
        assert_eq!(list.backend, LiveAudioInputBackendV1::WasapiShared);

        let capabilities = parse::<LiveAudioInputCapabilitiesRequestV1>(
            "get_live_audio_input_capabilities",
            json!({
                "request": {
                    "schemaVersion": 1,
                    "backend": "asio",
                    "deviceId": null,
                    "sampleRate": null
                }
            }),
        )
        .unwrap()
        .into_inner();
        assert_eq!(capabilities.device_id.into_option(), None);
        assert_eq!(capabilities.sample_rate.into_option(), None);

        let average_all = parse::<StartLiveAudioInputRequestV1>(
            "start_live_audio_input",
            json!({
                "request": {
                    "schemaVersion": 1,
                    "backend": "wasapiShared",
                    "deviceId": null,
                    "sampleRate": null,
                    "streamChannels": null,
                    "sampleFormat": null,
                    "bufferFrames": null,
                    "channelMix": { "mode": "averageAll" }
                }
            }),
        )
        .unwrap()
        .into_inner();
        assert_eq!(average_all.channel_mix, LiveAudioChannelMixV1::AverageAll);

        let single = parse::<StartLiveAudioInputRequestV1>(
            "start_live_audio_input",
            json!({
                "request": {
                    "schemaVersion": 1,
                    "backend": "wasapiShared",
                    "deviceId": null,
                    "sampleRate": 48000,
                    "streamChannels": 2,
                    "sampleFormat": "f32",
                    "bufferFrames": 128,
                    "channelMix": { "mode": "single", "channelIndex": 1 }
                }
            }),
        )
        .unwrap()
        .into_inner();
        assert_eq!(
            single.channel_mix,
            LiveAudioChannelMixV1::Single { channel_index: 1 }
        );

        let stereo_pair = parse::<StartLiveAudioInputRequestV1>(
            "start_live_audio_input",
            json!({
                "request": {
                    "schemaVersion": 1,
                    "backend": "wasapiShared",
                    "deviceId": null,
                    "sampleRate": null,
                    "streamChannels": null,
                    "sampleFormat": null,
                    "bufferFrames": null,
                    "channelMix": {
                        "mode": "stereoPair",
                        "leftChannelIndex": 0,
                        "rightChannelIndex": 1
                    }
                }
            }),
        )
        .unwrap()
        .into_inner();
        assert_eq!(
            stereo_pair.channel_mix,
            LiveAudioChannelMixV1::StereoPair {
                left_channel_index: 0,
                right_channel_index: 1,
            }
        );
    }

    #[test]
    fn v1_preflight_rejects_schema_outer_unknown_and_legacy_shapes() {
        let valid_start = json!({
            "schemaVersion": 1,
            "backend": "wasapiShared",
            "deviceId": null,
            "sampleRate": null,
            "streamChannels": null,
            "sampleFormat": null,
            "bufferFrames": null,
            "channelMix": { "mode": "averageAll" }
        });
        let invalid = [
            json!({ "request": { "backend": "wasapiShared" } }),
            json!({ "request": { "schemaVersion": 2, "backend": "wasapiShared" } }),
            json!({ "request": { "schemaVersion": 1, "backend": "wasapiShared", "unknown": true } }),
            json!({ "request": valid_start, "extra": true }),
            valid_start.clone(),
            json!({
                "request": {
                    "schema_version": 1,
                    "backend": "wasapi_shared",
                    "device_id": null,
                    "sample_rate": null,
                    "stream_channels": null,
                    "sample_format": null,
                    "buffer_frames": null,
                    "channel_mix": { "mode": "average_all" }
                }
            }),
        ];

        for value in invalid {
            assert!(admit_live_audio_ipc_v1_invoke_body(
                "start_live_audio_input",
                &InvokeBody::Json(value),
            )
            .is_err());
        }
        assert!(admit_live_audio_ipc_v1_invoke_body(
            "start_live_audio_input",
            &InvokeBody::Raw(vec![0x00]),
        )
        .is_err());
    }

    #[test]
    fn v1_requires_every_nullable_field_even_when_null_is_valid() {
        let capability_base = json!({
            "schemaVersion": 1,
            "backend": "wasapiShared",
            "deviceId": null,
            "sampleRate": null
        });
        for field in ["deviceId", "sampleRate"] {
            let mut request = capability_base.clone();
            request.as_object_mut().unwrap().remove(field);
            assert!(
                parse::<LiveAudioInputCapabilitiesRequestV1>(
                    "get_live_audio_input_capabilities",
                    json!({ "request": request }),
                )
                .is_err(),
                "omitting {field} must not be treated as null"
            );
        }

        let start_base = json!({
            "schemaVersion": 1,
            "backend": "wasapiShared",
            "deviceId": null,
            "sampleRate": null,
            "streamChannels": null,
            "sampleFormat": null,
            "bufferFrames": null,
            "channelMix": { "mode": "averageAll" }
        });
        for field in [
            "deviceId",
            "sampleRate",
            "streamChannels",
            "sampleFormat",
            "bufferFrames",
        ] {
            let mut request = start_base.clone();
            request.as_object_mut().unwrap().remove(field);
            assert!(
                parse::<StartLiveAudioInputRequestV1>(
                    "start_live_audio_input",
                    json!({ "request": request }),
                )
                .is_err(),
                "omitting {field} must not be treated as null"
            );
        }
    }

    #[test]
    fn v1_rejects_invalid_channel_mix_variants_and_fields() {
        let base = json!({
            "schemaVersion": 1,
            "backend": "wasapiShared",
            "deviceId": null,
            "sampleRate": null,
            "streamChannels": null,
            "sampleFormat": null,
            "bufferFrames": null
        });
        let invalid_mix = [
            json!({ "mode": "average_all" }),
            json!({ "mode": "single" }),
            json!({ "mode": "single", "channel_index": 0 }),
            json!({ "mode": "averageAll", "channelIndex": 0 }),
            json!({ "mode": "stereoPair", "leftChannelIndex": 0 }),
            json!({ "mode": "stereoPair", "leftChannelIndex": 0, "rightChannelIndex": 1, "channelIndex": 0 }),
            json!({ "mode": "quadPair", "leftChannelIndex": 0, "rightChannelIndex": 1 }),
        ];
        for mix in invalid_mix {
            let mut request = base.clone();
            request
                .as_object_mut()
                .unwrap()
                .insert("channelMix".to_string(), mix);
            assert!(parse::<StartLiveAudioInputRequestV1>(
                "start_live_audio_input",
                json!({ "request": request }),
            )
            .is_err());
        }
    }
}
