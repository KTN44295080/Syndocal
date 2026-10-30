//! External-route reconciliation needs inventory only when an authored route
//! consumes it. File decoding and native Display are not external transports.
//! In particular, Standby must not spawn ten unrelated CLI probes while it
//! waits to retire a file-only Display. Empty plans still reach reconciliation
//! so previously active (now removed) routes are stopped and acknowledged.

use super::*;

pub fn build_external_video_io_route_plans_with_runtime_status(
    snapshot: &VideoSnapshot,
) -> ExternalVideoIoRoutePlans {
    route_plans_with_inventory(snapshot, video_runtime_status)
}

fn route_plans_with_inventory(
    snapshot: &VideoSnapshot,
    inventory: impl FnOnce() -> VideoRuntimeStatus,
) -> ExternalVideoIoRoutePlans {
    if build_external_video_input_plans(snapshot).is_empty()
        && build_external_video_output_plans(snapshot).is_empty()
    {
        return ExternalVideoIoRoutePlans {
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
    }
    build_external_video_io_route_plans(snapshot, &inventory())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn snapshot() -> VideoSnapshot {
        VideoSnapshot {
            layers: Vec::new(),
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: protocol::AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        }
    }

    fn layer(kind: VideoSourceKind) -> VideoLayerSummary {
        VideoLayerSummary {
            id: 1,
            label: "source".into(),
            source: VideoSourceSummary {
                kind,
                path: Some("source.mp4".into()),
                name: Some("endpoint".into()),
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        }
    }

    fn output(kind: VideoOutputKind) -> VideoOutputSummary {
        VideoOutputSummary {
            id: 1,
            label: "output".into(),
            kind,
            enabled: true,
            composition_id: 1,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 2560,
            height: 1600,
            endpoint_name: Some("endpoint".into()),
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }
    }

    #[test]
    fn file_display_stop_never_probes_cli_inventory() {
        let mut snapshot = snapshot();
        snapshot.layers = vec![layer(VideoSourceKind::File); 3];
        snapshot.outputs.push(output(VideoOutputKind::Display));
        let plans = route_plans_with_inventory(&snapshot, || {
            panic!("file/Display Stop must not launch a CLI inventory probe")
        });
        assert!(plans.inputs.is_empty() && plans.outputs.is_empty());
    }

    #[test]
    fn external_routes_keep_the_exact_inventory_plan_including_denied_backends() {
        let status = VideoRuntimeStatus {
            backends: vec![
                VideoBackendStatus {
                    id: "camera".into(),
                    label: "camera".into(),
                    state: VideoBackendState::Available,
                    detail: "available".into(),
                },
                VideoBackendStatus {
                    id: "spout".into(),
                    label: "spout".into(),
                    state: VideoBackendState::Missing,
                    detail: "missing".into(),
                },
            ],
        };
        for kind in [
            VideoSourceKind::Camera,
            VideoSourceKind::ScreenCapture,
            VideoSourceKind::Ndi,
            VideoSourceKind::Spout,
            VideoSourceKind::Syphon,
        ] {
            let mut snapshot = snapshot();
            snapshot.layers.push(layer(kind));
            for enabled in [true, false] {
                snapshot.layers[0].state.enabled = enabled;
                assert_same_plan(&snapshot, &status);
            }
        }
        for kind in [
            VideoOutputKind::NdiSender,
            VideoOutputKind::SpoutSender,
            VideoOutputKind::SyphonServer,
        ] {
            let mut snapshot = snapshot();
            snapshot.outputs.push(output(kind));
            for enabled in [true, false] {
                for blackout in [true, false] {
                    snapshot.outputs[0].enabled = enabled;
                    snapshot.outputs[0].blackout = blackout;
                    assert_same_plan(&snapshot, &status);
                }
            }
        }
    }

    fn assert_same_plan(snapshot: &VideoSnapshot, status: &VideoRuntimeStatus) {
        let calls = Cell::new(0);
        let actual = route_plans_with_inventory(snapshot, || {
            calls.set(calls.get() + 1);
            status.clone()
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(
            actual,
            build_external_video_io_route_plans(snapshot, status)
        );
    }

    #[test]
    fn empty_plans_still_reap_stale_routes_and_retain_failed_stops() {
        struct Driver(bool);
        impl ExternalVideoTransportDriver for Driver {
            fn start_route(
                &mut self,
                _: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                Ok(())
            }
            fn stop_route(
                &mut self,
                _: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                if self.0 {
                    Err(ExternalVideoTransportDriverError {
                        message: "stop not acknowledged".into(),
                    })
                } else {
                    Ok(())
                }
            }
        }
        let mut authored = snapshot();
        authored.outputs.push(output(VideoOutputKind::SpoutSender));
        let status = VideoRuntimeStatus {
            backends: vec![VideoBackendStatus {
                id: "spout".into(),
                label: "spout".into(),
                state: VideoBackendState::Available,
                detail: "available".into(),
            }],
        };
        let mut runtime = ExternalVideoTransportRuntime::new();
        let mut driver = Driver(false);
        let plans = route_plans_with_inventory(&authored, || status);
        assert_eq!(
            runtime
                .sync_routes_with_driver_and_role(&plans, &mut driver, MachineOutputRole::Video)
                .active_count,
            1
        );
        let empty =
            route_plans_with_inventory(&snapshot(), || panic!("no inventory for removed routes"));
        driver.0 = true;
        let failed = runtime.sync_routes_with_driver_and_role(
            &empty,
            &mut driver,
            MachineOutputRole::Standby,
        );
        assert_eq!(failed.stop_failed.len(), 1);
        assert_eq!(failed.active_count, 1, "failed stop retains its owner");
        driver.0 = false;
        let reaped = runtime.sync_routes_with_driver_and_role(
            &empty,
            &mut driver,
            MachineOutputRole::Standby,
        );
        assert_eq!(reaped.stopped.len(), 1);
        assert_eq!(reaped.active_count, 0);
    }
}
