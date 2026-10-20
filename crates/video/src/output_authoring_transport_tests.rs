use super::*;

#[derive(Default)]
struct Driver {
    starts: Vec<u64>,
    stops: Vec<u64>,
    fail_stop: bool,
}

impl ExternalVideoTransportDriver for Driver {
    fn start_route(
        &mut self,
        route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError> {
        self.starts.push(route.route_id);
        Ok(())
    }

    fn stop_route(
        &mut self,
        route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError> {
        self.stops.push(route.route_id);
        if self.fail_stop {
            Err(ExternalVideoTransportDriverError {
                message: "injected retirement ambiguity".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

fn plans(ids: &[u64]) -> ExternalVideoIoRoutePlans {
    ExternalVideoIoRoutePlans {
        inputs: Vec::new(),
        outputs: ids
            .iter()
            .map(|id| ExternalVideoOutputRoutePlan {
                output_id: *id,
                label: format!("Output {id}"),
                kind: VideoOutputKind::SpoutSender,
                backend_id: "spout".to_string(),
                backend_label: Some("Spout".to_string()),
                backend_state: Some(VideoBackendState::Available),
                backend_detail: None,
                endpoint_name: format!("Endpoint {id}"),
                enabled: true,
                width: 1280,
                height: 720,
                opacity: 1.0,
                blackout: false,
                composition_id: 1,
                ready: true,
                live: true,
                issue: None,
            })
            .collect(),
    }
}

#[test]
fn scoped_authoring_preserves_other_routes_and_does_not_retry_absent_outputs() {
    let mut runtime = ExternalVideoTransportRuntime::new();
    runtime.sync_routes(&plans(&[1, 2]));
    let input = ExternalVideoTransportRoute {
        direction: ExternalVideoTransportDirection::Input,
        route_id: 1,
        label: "Camera".to_string(),
        backend_id: "camera".to_string(),
        endpoint_name: "Camera A".to_string(),
    };
    runtime.active_routes.push(input.clone());
    let mut driver = Driver::default();
    let report = runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &plans(&[2, 3]),
        &mut driver,
        MachineOutputRole::Both,
        |_| Ok(()),
    );
    assert_eq!(driver.stops, [1]);
    assert!(driver.starts.is_empty());
    assert_eq!(report.active_count, 2);
    assert!(runtime.active_routes.contains(&input));
    assert!(runtime.active_routes.iter().any(|route| route.direction
        == ExternalVideoTransportDirection::Output
        && route.route_id == 2));
    assert!(!runtime.active_routes.iter().any(|route| route.direction
        == ExternalVideoTransportDirection::Output
        && route.route_id == 3));
}

#[test]
fn scoped_authoring_resize_retires_exactly_once_and_recreates_once() {
    let mut runtime = ExternalVideoTransportRuntime::new();
    runtime.sync_routes(&plans(&[1, 2]));
    let mut driver = Driver::default();
    runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &plans(&[2]),
        &mut driver,
        MachineOutputRole::Both,
        |_| Ok(()),
    );
    runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &plans(&[1, 2]),
        &mut driver,
        MachineOutputRole::Both,
        |_| Ok(()),
    );
    assert_eq!(driver.stops, [1]);
    assert_eq!(driver.starts, [1]);
    assert_eq!(runtime.active_routes.len(), 2);
}

#[test]
fn scoped_authoring_keeps_failed_retirement_owned_and_never_double_starts() {
    let mut runtime = ExternalVideoTransportRuntime::new();
    runtime.sync_routes(&plans(&[1, 2]));
    let mut driver = Driver {
        fail_stop: true,
        ..Driver::default()
    };
    let mut replacement = plans(&[1, 2]);
    replacement.outputs[0].endpoint_name = "Renamed".to_string();
    let report = runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &replacement,
        &mut driver,
        MachineOutputRole::Both,
        |_| Ok(()),
    );
    assert_eq!(driver.stops, [1]);
    assert!(driver.starts.is_empty());
    assert_eq!(report.stop_failed.len(), 1);
    assert_eq!(report.start_failed.len(), 1);
    assert_eq!(runtime.active_routes.len(), 2);
    assert!(runtime
        .active_routes
        .iter()
        .any(|route| route.endpoint_name == "Endpoint 1"));
}

#[test]
fn scoped_authoring_standby_and_expired_admission_never_start_a_sender() {
    let mut runtime = ExternalVideoTransportRuntime::new();
    let mut driver = Driver::default();
    runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &plans(&[1]),
        &mut driver,
        MachineOutputRole::Standby,
        |_| Ok(()),
    );
    assert!(driver.starts.is_empty());
    let report = runtime.sync_output_route_with_driver_and_role_with_start_admission(
        1,
        &plans(&[1]),
        &mut driver,
        MachineOutputRole::Both,
        |_| Err("lease expired before sender creation".to_string()),
    );
    assert!(driver.starts.is_empty());
    assert_eq!(report.start_failed.len(), 1);
    assert!(runtime.active_routes.is_empty());
}
