//! Focused E1 terminal-recovery integration proof.
//!
//! Keep this lifecycle sequence out of `main.rs`: it exercises the real
//! transaction receipt/lane helpers without adding another large inline test
//! to the production command module.

use super::*;
use crate::tests::{MediaAssetA6CommandHarness, MEDIA_ASSET_A6_OWNER};

const WINDOW_LABEL: &str = "media-asset-a6";
const COMMAND_NAME: &str = "terminal_recovery_proof";

fn begin_ticket(state: &AppState, operation_id: &str, label: &str) -> ProjectTransactionTicket {
    let (epoch, revision, checkpoint_hash) = {
        let coordinator = lock_project_coordinator(state).expect("lock project coordinator");
        (
            coordinator.epoch,
            coordinator.revision,
            coordinator.checkpoint_hash.clone(),
        )
    };
    let coalesce_key = "terminal-recovery-proof";
    begin_project_transaction_for_window_label(
        state,
        WINDOW_LABEL,
        BeginProjectTransactionRequest {
            label: label.to_string(),
            coalesce_key: coalesce_key.to_string(),
            expected_epoch: epoch,
            expected_revision: revision,
            expected_checkpoint_hash: checkpoint_hash,
            owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
            client_operation_id: operation_id.to_string(),
            shape_fingerprint: canonical_project_transaction_shape(
                COMMAND_NAME,
                label,
                coalesce_key,
            ),
            command_name: COMMAND_NAME.to_string(),
            schema_version: PROJECT_TRANSACTION_SCHEMA_VERSION,
        },
    )
    .expect("begin project transaction")
}

fn commit_request(ticket: &ProjectTransactionTicket) -> CommitProjectTransactionRequest {
    CommitProjectTransactionRequest {
        transaction_id: ticket.transaction_id,
        expected_epoch: ticket.project_epoch,
        client_operation_id: ticket.client_operation_id.clone(),
        shape_fingerprint: ticket.shape_fingerprint.clone(),
        command_name: COMMAND_NAME.to_string(),
        schema_version: ticket.schema_version,
        owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
    }
}

fn cancel_request(ticket: &ProjectTransactionTicket) -> ProjectTransactionCancelRequest {
    ProjectTransactionCancelRequest {
        window_label: WINDOW_LABEL.to_string(),
        transaction_id: ticket.transaction_id,
        expected_epoch: ticket.project_epoch,
        client_operation_id: ticket.client_operation_id.clone(),
        shape_fingerprint: ticket.shape_fingerprint.clone(),
        command_name: COMMAND_NAME.to_string(),
        schema_version: ticket.schema_version,
        owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
    }
}

fn query_ticket(
    state: &AppState,
    ticket: &ProjectTransactionTicket,
) -> Result<Option<ProjectTransactionRecovery>, String> {
    query_project_transaction_for_window_label(
        state,
        WINDOW_LABEL,
        ticket.client_operation_id.clone(),
        ticket.shape_fingerprint.clone(),
        COMMAND_NAME.to_string(),
        ticket.schema_version,
        MEDIA_ASSET_A6_OWNER.to_string(),
    )
}

fn acknowledge_ticket(state: &AppState, ticket: &ProjectTransactionTicket) {
    acknowledge_project_transaction_for_window_label(
        state,
        WINDOW_LABEL,
        ticket.client_operation_id.clone(),
        ticket.shape_fingerprint.clone(),
        COMMAND_NAME.to_string(),
        ticket.schema_version,
        MEDIA_ASSET_A6_OWNER.to_string(),
    )
    .expect("acknowledge exact terminal receipt");
}

#[test]
fn terminal_error_worker_completion_same_ticket_recovery_ack_and_fresh_commit() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = Arc::clone(&harness.state);

    let stranded_ticket = begin_ticket(
        &state,
        "project-op:1:terminal-recovery-proof",
        "Terminal recovery proof",
    );
    let lane = project_transaction_lane_for_operation(&state, &stranded_ticket.client_operation_id)
        .expect("opened ticket owns one operation lane");
    let (worker_ready_tx, worker_ready_rx) = std::sync::mpsc::sync_channel(1);
    let (worker_finish_tx, worker_finish_rx) = std::sync::mpsc::sync_channel(1);

    std::thread::scope(|scope| {
        let worker_lane = Arc::clone(&lane);
        let worker = scope.spawn(move || {
            let admitted = worker_lane
                .admit()
                .expect("worker command enters its exact lane");
            worker_ready_tx.send(()).expect("announce admitted worker");
            worker_finish_rx.recv().expect("release worker completion");
            drop(admitted);
        });
        worker_ready_rx
            .recv()
            .expect("wait until worker is in flight");

        let terminal_error = commit_project_transaction_for_window_label(
            &state,
            WINDOW_LABEL,
            commit_request(&stranded_ticket),
        )
        .expect_err("Commit must fail while an admitted worker is still in flight");
        assert!(
            terminal_error.contains("admitted commands"),
            "unexpected Commit terminal error: {terminal_error}"
        );
        assert!(
            state.project_transaction_active.load(Ordering::Acquire),
            "the failed terminal must retain its exact pending receipt for recovery"
        );
        assert!(matches!(
            query_ticket(&state, &stranded_ticket).expect("query in-flight receipt"),
            Some(ProjectTransactionRecovery::Pending {
                command_result: None,
                command_in_flight: true,
                command_indeterminate_error: None,
                ..
            })
        ));

        worker_finish_tx.send(()).expect("allow worker completion");
        worker.join().expect("worker completion must not panic");
    });

    // The first Commit's lane-close fence remains in force, but the worker is
    // now complete. Recovery may terminalize only this retained ticket; it
    // must not issue a second project mutation or a fresh Begin.
    assert!(matches!(
        query_ticket(&state, &stranded_ticket).expect("query completed worker receipt"),
        Some(ProjectTransactionRecovery::Pending {
            command_result: None,
            command_in_flight: false,
            command_indeterminate_error: None,
            ..
        })
    ));
    let cancelled =
        cancel_project_transaction_for_window_label(&state, cancel_request(&stranded_ticket))
            .expect("same ticket Cancel terminalizes the completed worker receipt");
    assert_eq!(cancelled.history_status.undo_depth, 0);
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
    assert!(matches!(
        query_ticket(&state, &stranded_ticket).expect("query cancelled receipt"),
        Some(ProjectTransactionRecovery::Cancelled { .. })
    ));

    acknowledge_ticket(&state, &stranded_ticket);
    assert!(
        query_ticket(&state, &stranded_ticket).is_err(),
        "ACK compacts the receipt and preserves its replay high-water fence"
    );
    assert!(!state
        .project_transaction_lanes
        .lock()
        .expect("lock lane registry")
        .contains_key(&stranded_ticket.client_operation_id));

    // A different, monotonic operation proves that the recovered terminal did
    // not leave active/pending/closing state behind.
    let fresh_ticket = begin_ticket(
        &state,
        "project-op:2:terminal-recovery-proof",
        "Fresh transaction after recovery",
    );
    assert!(fresh_ticket.transaction_id > stranded_ticket.transaction_id);
    commit_project_transaction_for_window_label(
        &state,
        WINDOW_LABEL,
        commit_request(&fresh_ticket),
    )
    .expect("fresh Begin/Commit succeeds after terminal recovery");
    acknowledge_ticket(&state, &fresh_ticket);
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
    assert!(lock_project_coordinator(&state)
        .expect("lock coordinator after fresh terminal")
        .history
        .pending
        .is_empty());
}
