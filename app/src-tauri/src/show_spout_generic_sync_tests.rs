    use super::*;
    use protocol::{VideoBackendState, VideoSourceKind};
    use video::{
        ExternalVideoInputRoutePlan, ExternalVideoOutputRoutePlan,
        ExternalVideoTransportDriver, ExternalVideoTransportDriverError,
        ExternalVideoTransportRoute, ExternalVideoTransportRuntime,
    };

    fn pair_snapshot(enabled: bool) -> (Vec<VideoOutputSummary>, Vec<CompositionSummary>) {
        let expected = crate::show_spout_outputs::build_show_spout_outputs(41, 42, 3, 2)
            .expect("test pair should be constructible");
        let mut outputs = vec![expected.background, expected.foreground];
        for output in &mut outputs {
            output.enabled = enabled;
        }
        let compositions = vec![
            CompositionSummary {
                id: crate::show_spout_outputs::SHOW_SPOUT_MAIN_COMPOSITION_ID,
                label: crate::show_spout_outputs::SHOW_SPOUT_MAIN_COMPOSITION_LABEL.to_string(),
                layer_ids: Vec::new(),
                timeline_layer_ids: Vec::new(),
                output_ids: Vec::new(),
            },
            CompositionSummary {
                id: 2,
                label: crate::show_spout_outputs::SHOW_SPOUT_FOREGROUND_COMPOSITION_LABEL
                    .to_string(),
                layer_ids: Vec::new(),
                timeline_layer_ids: Vec::new(),
                output_ids: vec![42],
            },
            CompositionSummary {
                id: 3,
                label: crate::show_spout_outputs::SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL
                    .to_string(),
                layer_ids: Vec::new(),
                timeline_layer_ids: Vec::new(),
                output_ids: vec![41],
            },
        ];
        (outputs, compositions)
    }

    fn output_plan(output: &VideoOutputSummary) -> ExternalVideoOutputRoutePlan {
        ExternalVideoOutputRoutePlan {
            output_id: output.id,
            label: output.label.clone(),
            kind: output.kind.clone(),
            backend_id: "spout".to_string(),
            backend_label: Some("Spout".to_string()),
            backend_state: Some(VideoBackendState::Available),
            backend_detail: Some("test backend".to_string()),
            endpoint_name: output
                .endpoint_name
                .clone()
                .expect("fixed pair has endpoint names"),
            enabled: output.enabled,
            width: output.width,
            height: output.height,
            opacity: output.opacity,
            blackout: output.blackout,
            composition_id: output.composition_id,
            ready: true,
            live: output.enabled && !output.blackout && output.opacity > 0.0,
            issue: None,
        }
    }

    fn ndi_input_plan() -> ExternalVideoInputRoutePlan {
        ExternalVideoInputRoutePlan {
            layer_id: 7,
            label: "NDI Camera".to_string(),
            kind: VideoSourceKind::Ndi,
            backend_id: "ndi".to_string(),
            backend_label: Some("NDI".to_string()),
            backend_state: Some(VideoBackendState::Available),
            backend_detail: Some("test backend".to_string()),
            endpoint_name: "Camera".to_string(),
            enabled: true,
            ready: false,
            live: false,
            issue: Some("test backend unavailable".to_string()),
        }
    }

    fn ndi_output_plan() -> ExternalVideoOutputRoutePlan {
        ExternalVideoOutputRoutePlan {
            output_id: 77,
            label: "NDI Program".to_string(),
            kind: VideoOutputKind::NdiSender,
            backend_id: "ndi".to_string(),
            backend_label: Some("NDI".to_string()),
            backend_state: Some(VideoBackendState::NotBuilt),
            backend_detail: Some("test backend unavailable".to_string()),
            endpoint_name: "NDI Program".to_string(),
            enabled: true,
            width: 1920,
            height: 1080,
            opacity: 1.0,
            blackout: false,
            composition_id: 1,
            ready: false,
            live: false,
            issue: Some("test backend unavailable".to_string()),
        }
    }

    fn plans_for(outputs: &[VideoOutputSummary]) -> ExternalVideoIoRoutePlans {
        let mut output_plans = outputs.iter().map(output_plan).collect::<Vec<_>>();
        output_plans.push(ndi_output_plan());
        ExternalVideoIoRoutePlans {
            inputs: vec![ndi_input_plan()],
            outputs: output_plans,
        }
    }

    #[derive(Default)]
    struct RecordingTransportDriver {
        started: Vec<ExternalVideoTransportRoute>,
        stopped: Vec<ExternalVideoTransportRoute>,
    }

    impl ExternalVideoTransportDriver for RecordingTransportDriver {
        fn start_route(
            &mut self,
            route: &ExternalVideoTransportRoute,
        ) -> Result<(), ExternalVideoTransportDriverError> {
            self.started.push(route.clone());
            Ok(())
        }

        fn stop_route(
            &mut self,
            route: &ExternalVideoTransportRoute,
        ) -> Result<(), ExternalVideoTransportDriverError> {
            self.stopped.push(route.clone());
            Ok(())
        }
    }

    #[test]
    fn generic_spout_sync_allows_disabled_pair_to_retire_without_activation() {
        let (enabled_outputs, compositions) = pair_snapshot(true);
        let enabled_plans = plans_for(&enabled_outputs);
        let filtered_enabled = filter_generic_spout_sync_plans(
            enabled_plans.clone(),
            &enabled_outputs,
            &compositions,
        )
        .expect("active exact pair is valid");
        assert!(filtered_enabled
            .outputs
            .iter()
            .all(|plan| plan.kind != VideoOutputKind::SpoutSender));
        assert_eq!(filtered_enabled.inputs, enabled_plans.inputs);
        assert_eq!(
            filtered_enabled
                .outputs
                .iter()
                .map(|plan| plan.kind.clone())
                .collect::<Vec<_>>(),
            vec![VideoOutputKind::NdiSender]
        );

        // Seed the generic registry with the pair, then pass the fully
        // disabled authored pair through the policy. The runtime must stop
        // those stale routes and must not start them again.
        let mut runtime = ExternalVideoTransportRuntime::new();
        let mut driver = RecordingTransportDriver::default();
        let seeded = runtime.sync_routes_with_driver(&enabled_plans, &mut driver);
        assert_eq!(
            seeded
                .started
                .iter()
                .map(|route| route.route_id)
                .collect::<Vec<_>>(),
            vec![41, 42]
        );

        let (disabled_outputs, disabled_compositions) = pair_snapshot(false);
        let disabled_plans = plans_for(&disabled_outputs);
        let filtered_disabled = filter_generic_spout_sync_plans(
            disabled_plans.clone(),
            &disabled_outputs,
            &disabled_compositions,
        )
        .expect("fully disabled exact pair is a cleanup candidate");
        assert_eq!(filtered_disabled.inputs, disabled_plans.inputs);
        assert!(filtered_disabled
            .outputs
            .iter()
            .any(|plan| plan.kind == VideoOutputKind::SpoutSender && !plan.live));
        assert!(filtered_disabled
            .outputs
            .iter()
            .any(|plan| plan.kind == VideoOutputKind::NdiSender));
        assert!(disabled_outputs.iter().all(|output| !output.enabled));

        let retired = runtime.sync_routes_with_driver(&filtered_disabled, &mut driver);
        assert!(retired.started.is_empty());
        assert_eq!(
            retired
                .stopped
                .iter()
                .map(|route| route.route_id)
                .collect::<Vec<_>>(),
            vec![41, 42]
        );
        assert_eq!(retired.active_count, 0);
        assert_eq!(driver.started.len(), 2);
        assert_eq!(driver.stopped.len(), 2);
    }

    #[test]
    fn generic_spout_sync_rejects_invalid_enabled_or_disabled_pairs() {
        let (mut enabled_invalid, compositions) = pair_snapshot(true);
        enabled_invalid[0].mapping.offset_x = 0.25;
        let error = filter_generic_spout_sync_plans(
            plans_for(&enabled_invalid),
            &enabled_invalid,
            &compositions,
        )
        .expect_err("enabled pair with a non-default mapping must fail closed");
        assert!(error.contains("invalid or conflicting strict show pair"));
        assert!(error.contains("mapping"));

        let (mut disabled_invalid_mapping, compositions) = pair_snapshot(false);
        disabled_invalid_mapping[0].mapping.offset_x = 0.25;
        assert!(filter_generic_spout_sync_plans(
            plans_for(&disabled_invalid_mapping),
            &disabled_invalid_mapping,
            &compositions,
        )
        .is_err());

        let (disabled_missing_backreference, mut compositions) = pair_snapshot(false);
        compositions
            .iter_mut()
            .find(|composition| {
                composition.label
                    == crate::show_spout_outputs::SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL
            })
            .expect("background composition exists")
            .output_ids
            .clear();
        assert!(filter_generic_spout_sync_plans(
            plans_for(&disabled_missing_backreference),
            &disabled_missing_backreference,
            &compositions,
        )
        .is_err());

        let (mut disabled_extra, compositions) = pair_snapshot(false);
        let mut extra = disabled_extra[0].clone();
        extra.id = 99;
        extra.label = "Extra sender".to_string();
        extra.endpoint_name = Some("Extra sender".to_string());
        disabled_extra.push(extra);
        assert!(filter_generic_spout_sync_plans(
            plans_for(&disabled_extra),
            &disabled_extra,
            &compositions,
        )
        .is_err());

        let (mut mixed, compositions) = pair_snapshot(false);
        mixed[0].enabled = true;
        assert!(filter_generic_spout_sync_plans(
            plans_for(&mixed),
            &mixed,
            &compositions,
        )
        .is_err());
    }

    #[test]
    fn generic_spout_sync_does_not_mutate_disabled_pair_during_validation() {
        let (outputs, compositions) = pair_snapshot(false);
        let plans = plans_for(&outputs);
        let filtered = filter_generic_spout_sync_plans(plans.clone(), &outputs, &compositions)
            .expect("fully disabled exact pair is valid");
        assert_eq!(outputs, pair_snapshot(false).0);
        assert_eq!(filtered, plans);
    }
