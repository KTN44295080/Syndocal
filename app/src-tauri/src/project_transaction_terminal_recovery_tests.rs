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
    begin_ticket_for(
        state,
        WINDOW_LABEL,
        MEDIA_ASSET_A6_OWNER,
        operation_id,
        label,
    )
}

fn begin_ticket_for(
    state: &AppState,
    window_label: &str,
    owner_id: &str,
    operation_id: &str,
    label: &str,
) -> ProjectTransactionTicket {
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
        window_label,
        BeginProjectTransactionRequest {
            label: label.to_string(),
            coalesce_key: coalesce_key.to_string(),
            expected_epoch: epoch,
            expected_revision: revision,
            expected_checkpoint_hash: checkpoint_hash,
            owner_id: owner_id.to_string(),
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
        owner_id: ticket.owner_id.clone(),
    }
}

fn cancel_request(ticket: &ProjectTransactionTicket) -> ProjectTransactionCancelRequest {
    ProjectTransactionCancelRequest {
        window_label: ticket.window_label.clone(),
        transaction_id: ticket.transaction_id,
        expected_epoch: ticket.project_epoch,
        client_operation_id: ticket.client_operation_id.clone(),
        shape_fingerprint: ticket.shape_fingerprint.clone(),
        command_name: COMMAND_NAME.to_string(),
        schema_version: ticket.schema_version,
        owner_id: ticket.owner_id.clone(),
    }
}

fn query_ticket(
    state: &AppState,
    ticket: &ProjectTransactionTicket,
) -> Result<Option<ProjectTransactionRecovery>, String> {
    query_project_transaction_for_window_label(
        state,
        &ticket.window_label,
        ticket.client_operation_id.clone(),
        ticket.shape_fingerprint.clone(),
        COMMAND_NAME.to_string(),
        ticket.schema_version,
        ticket.owner_id.clone(),
    )
}

fn acknowledge_ticket(state: &AppState, ticket: &ProjectTransactionTicket) {
    acknowledge_project_transaction_for_window_label(
        state,
        &ticket.window_label,
        ticket.client_operation_id.clone(),
        ticket.shape_fingerprint.clone(),
        COMMAND_NAME.to_string(),
        ticket.schema_version,
        ticket.owner_id.clone(),
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

        let cancel_error =
            cancel_project_transaction_for_window_label(&state, cancel_request(&stranded_ticket))
                .expect_err("Cancel must also fail while the exact worker remains in flight");
        assert!(
            cancel_error.contains("admitted commands"),
            "unexpected Cancel terminal error: {cancel_error}"
        );
        assert!(matches!(
            query_ticket(&state, &stranded_ticket).expect("query after failed Cancel"),
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

#[test]
fn owner_replacement_compacts_terminal_receipts_and_recovers_terminal_capacity() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = Arc::clone(&harness.state);
    let successor_owner = "renderer:terminal-capacity-successor";

    for sequence in 1..=MAX_PROJECT_TRANSACTION_TERMINAL_RECEIPTS {
        let ticket = begin_ticket(
            &state,
            &format!("project-op:{sequence}:terminal-capacity"),
            "Terminal capacity receipt",
        );
        if sequence % 2 == 0 {
            commit_project_transaction_for_window_label(
                &state,
                WINDOW_LABEL,
                commit_request(&ticket),
            )
            .expect("Commit leaves one unacknowledged terminal receipt");
        } else {
            cancel_project_transaction_for_window_label(&state, cancel_request(&ticket))
                .expect("Cancel leaves one unacknowledged terminal receipt");
        }
    }

    let terminal_count = state
        .project_transaction_receipts
        .lock()
        .expect("lock terminal receipts")
        .values()
        .filter(|receipt| {
            receipt.owner_id == MEDIA_ASSET_A6_OWNER
                && receipt.window_label == WINDOW_LABEL
                && matches!(
                    receipt.state,
                    ProjectTransactionReceiptState::Committed(_)
                        | ProjectTransactionReceiptState::Cancelled(_)
                )
        })
        .count();
    assert_eq!(terminal_count, MAX_PROJECT_TRANSACTION_TERMINAL_RECEIPTS);

    let (epoch, revision, checkpoint_hash) = {
        let coordinator = lock_project_coordinator(&state).expect("lock coordinator at capacity");
        (
            coordinator.epoch,
            coordinator.revision,
            coordinator.checkpoint_hash.clone(),
        )
    };
    let overflow_label = "Terminal capacity overflow";
    let overflow_coalesce_key = "terminal-recovery-proof";
    let overflow_error = begin_project_transaction_for_window_label(
        &state,
        WINDOW_LABEL,
        BeginProjectTransactionRequest {
            label: overflow_label.to_string(),
            coalesce_key: overflow_coalesce_key.to_string(),
            expected_epoch: epoch,
            expected_revision: revision,
            expected_checkpoint_hash: checkpoint_hash,
            owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
            client_operation_id: "project-op:999:terminal-capacity-overflow".to_string(),
            shape_fingerprint: canonical_project_transaction_shape(
                COMMAND_NAME,
                overflow_label,
                overflow_coalesce_key,
            ),
            command_name: COMMAND_NAME.to_string(),
            schema_version: PROJECT_TRANSACTION_SCHEMA_VERSION,
        },
    )
    .expect_err("unacknowledged terminal receipts must fill the admission cap");
    assert!(overflow_error.contains("terminal receipt capacity is full"));

    let retired_incarnation = state
        .project_transaction_owner_incarnations
        .lock()
        .expect("lock owner incarnations before replacement")
        .get(WINDOW_LABEL)
        .copied()
        .expect("main owner has an incarnation");
    register_project_transaction_owner_for_window_label(
        &state,
        WINDOW_LABEL,
        successor_owner.to_string(),
    )
    .expect("successful owner replacement compacts unreachable terminal receipts");

    assert!(state
        .project_transaction_receipts
        .lock()
        .expect("lock compacted terminal receipts")
        .values()
        .all(|receipt| {
            !(receipt.owner_id == MEDIA_ASSET_A6_OWNER
                && receipt.window_label == WINDOW_LABEL
                && receipt.owner_incarnation == retired_incarnation)
        }));
    assert!(state
        .project_transaction_lanes
        .lock()
        .expect("lock compacted terminal lanes")
        .is_empty());

    let fresh_ticket = begin_ticket_for(
        &state,
        WINDOW_LABEL,
        successor_owner,
        "project-op:1:terminal-capacity-successor",
        "Fresh transaction after terminal compaction",
    );
    commit_project_transaction_for_window_label(
        &state,
        WINDOW_LABEL,
        commit_request(&fresh_ticket),
    )
    .expect("new owner admits a transaction after capacity compaction");
    acknowledge_ticket(&state, &fresh_ticket);
}

#[test]
fn destroyed_owner_compacts_only_its_exact_terminal_receipts() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = Arc::clone(&harness.state);
    let pane_label = "pane-terminal-recovery";
    let pane_owner = "renderer:pane-terminal-recovery";
    register_project_transaction_owner_for_window_label(&state, pane_label, pane_owner.to_string())
        .expect("register independent pane owner");

    let main_committed = begin_ticket(
        &state,
        "project-op:1:destroyed-main-commit",
        "Destroyed main Commit",
    );
    commit_project_transaction_for_window_label(
        &state,
        WINDOW_LABEL,
        commit_request(&main_committed),
    )
    .expect("main Commit terminalizes before its ACK reply is lost");
    let main_cancelled = begin_ticket(
        &state,
        "project-op:2:destroyed-main-cancel",
        "Destroyed main Cancel",
    );
    cancel_project_transaction_for_window_label(&state, cancel_request(&main_cancelled))
        .expect("main Cancel terminalizes before its ACK reply is lost");
    let pane_ticket = begin_ticket_for(
        &state,
        pane_label,
        pane_owner,
        "project-op:1:destroyed-pane-commit",
        "Live pane Commit",
    );
    commit_project_transaction_for_window_label(&state, pane_label, commit_request(&pane_ticket))
        .expect("live pane terminalizes independently");

    let query = ControlPlaneQueryState::new().expect("query state initializes");
    handle_destroyed_window_authority_retirement(&state, &query, WINDOW_LABEL)
        .expect("Destroyed owner retirement compacts its exact terminal receipts");

    let receipts = state
        .project_transaction_receipts
        .lock()
        .expect("lock receipts after Destroyed retirement");
    assert!(!receipts.contains_key(&main_committed.client_operation_id));
    assert!(!receipts.contains_key(&main_cancelled.client_operation_id));
    assert!(receipts.contains_key(&pane_ticket.client_operation_id));
    drop(receipts);
    let lanes = state
        .project_transaction_lanes
        .lock()
        .expect("lock lanes after Destroyed retirement");
    assert!(!lanes.contains_key(&main_committed.client_operation_id));
    assert!(!lanes.contains_key(&main_cancelled.client_operation_id));
    assert!(lanes.contains_key(&pane_ticket.client_operation_id));
    drop(lanes);

    acknowledge_ticket(&state, &pane_ticket);
}

#[test]
fn owner_replacement_fails_closed_when_a_terminal_receipt_loses_its_exact_lane() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = Arc::clone(&harness.state);
    let ticket = begin_ticket(
        &state,
        "project-op:1:missing-terminal-lane",
        "Terminal receipt with missing lane",
    );
    commit_project_transaction_for_window_label(&state, WINDOW_LABEL, commit_request(&ticket))
        .expect("create terminal receipt before corrupting its lane registry");
    state
        .project_transaction_lanes
        .lock()
        .expect("lock lane registry")
        .remove(&ticket.client_operation_id)
        .expect("terminal receipt initially owns one closed lane");

    let error = register_project_transaction_owner_for_window_label(
        &state,
        WINDOW_LABEL,
        "renderer:missing-terminal-lane-successor".to_string(),
    )
    .expect_err("missing terminal lane must fail closed before owner replacement");
    assert!(
        error.contains("missing its command lane"),
        "unexpected error: {error}"
    );
    assert_eq!(
        state
            .project_transaction_owners
            .lock()
            .expect("lock owners after failed replacement")
            .get(WINDOW_LABEL)
            .map(String::as_str),
        Some(MEDIA_ASSET_A6_OWNER)
    );
    assert!(state
        .project_transaction_receipts
        .lock()
        .expect("lock receipts after failed replacement")
        .contains_key(&ticket.client_operation_id));
}

#[test]
fn failed_owner_replacement_preserves_terminal_and_indeterminate_pending_receipts() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = Arc::clone(&harness.state);
    let terminal_ticket = begin_ticket(
        &state,
        "project-op:1:indeterminate-terminal",
        "Retained terminal before indeterminate pending",
    );
    commit_project_transaction_for_window_label(
        &state,
        WINDOW_LABEL,
        commit_request(&terminal_ticket),
    )
    .expect("create terminal receipt that must not compact before a failed retirement");
    let pending_ticket = begin_ticket(
        &state,
        "project-op:2:indeterminate-pending",
        "Indeterminate pending receipt",
    );
    let marker = "injected indeterminate publication".to_string();
    {
        let mut coordinator = lock_project_coordinator(&state).expect("lock coordinator");
        coordinator
            .history
            .pending
            .get_mut(&pending_ticket.transaction_id)
            .expect("pending transaction remains present")
            .command_indeterminate_error = Some(marker.clone());
    }
    {
        let mut receipts = state
            .project_transaction_receipts
            .lock()
            .expect("lock receipt registry");
        let pending_receipt = receipts
            .get_mut(&pending_ticket.client_operation_id)
            .expect("pending receipt remains present");
        assert!(matches!(
            pending_receipt.state,
            ProjectTransactionReceiptState::Pending
        ));
        pending_receipt.command_indeterminate_error = Some(marker.clone());
    }

    let error = register_project_transaction_owner_for_window_label(
        &state,
        WINDOW_LABEL,
        "renderer:indeterminate-successor".to_string(),
    )
    .expect_err("indeterminate Pending receipt must block owner replacement");
    assert!(
        error.contains("indeterminate"),
        "unexpected owner error: {error}"
    );
    assert_eq!(
        state
            .project_transaction_owners
            .lock()
            .expect("lock owners after failed replacement")
            .get(WINDOW_LABEL)
            .map(String::as_str),
        Some(MEDIA_ASSET_A6_OWNER)
    );
    let receipts = state
        .project_transaction_receipts
        .lock()
        .expect("lock receipts after failed replacement");
    assert_eq!(
        receipts
            .get(&terminal_ticket.client_operation_id)
            .expect("terminal receipt remains after failed retirement")
            .command_indeterminate_error
            .as_deref(),
        None
    );
    assert!(matches!(
        receipts
            .get(&terminal_ticket.client_operation_id)
            .expect("terminal receipt remains after failed retirement")
            .state,
        ProjectTransactionReceiptState::Committed(_)
    ));
    assert!(matches!(
        receipts
            .get(&pending_ticket.client_operation_id)
            .expect("indeterminate Pending receipt remains")
            .state,
        ProjectTransactionReceiptState::Pending
    ));
    assert_eq!(
        receipts
            .get(&pending_ticket.client_operation_id)
            .expect("indeterminate Pending receipt remains")
            .command_indeterminate_error
            .as_deref(),
        Some(marker.as_str())
    );
    drop(receipts);
    let lanes = state
        .project_transaction_lanes
        .lock()
        .expect("lock lanes after failed replacement");
    assert!(lanes.contains_key(&terminal_ticket.client_operation_id));
    assert!(lanes.contains_key(&pending_ticket.client_operation_id));
    drop(lanes);
    let coordinator =
        lock_project_coordinator(&state).expect("lock coordinator after failed retirement");
    assert_eq!(
        coordinator
            .history
            .pending
            .get(&pending_ticket.transaction_id)
            .expect("indeterminate Pending transaction remains")
            .command_indeterminate_error
            .as_deref(),
        Some(marker.as_str())
    );
}
