from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


runtime = Path("app/src-tauri/src/control_plane_runtime.rs")

# A new lane reserves one future terminal receipt slot. Do not admit more
# commands than the bounded receipt store can eventually retain.
replace_exact(
    runtime,
    '''        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::ReceiptExpired,
            );
        }
        if let Some(lane) = inner.lanes.get(key) {
            return OutputControlLaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded);
        }
''',
    '''        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            if record.expires_at <= now {
                return OutputControlLaneReservation::Rejected(
                    OutputControlErrorCodeV1::ReceiptExpired,
                );
            }
            return if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::ReceiptExpired,
            );
        }
        if let Some(lane) = inner.lanes.get(key) {
            return OutputControlLaneReservation::Lane(Arc::clone(lane));
        }
        let reserved_total = inner.receipts.len().saturating_add(inner.lanes.len());
        if reserved_total >= MAX_TOTAL_RECEIPTS {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded);
        }
        let reserved_for_principal_operation = inner
            .receipts
            .keys()
            .chain(inner.lanes.keys())
            .filter(|candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            })
            .count();
        if reserved_for_principal_operation >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded);
        }
        if inner.lanes.len() >= MAX_LANES {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded);
        }
''',
    "reserve bounded output terminal capacity at admission",
)

replace_exact(
    runtime,
    '''        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            });
        }
''',
    '''        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            if record.expires_at <= now {
                return Some(OutputControlLaneReservation::Rejected(
                    OutputControlErrorCodeV1::ReceiptExpired,
                ));
            }
            return Some(if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            });
        }
''',
    "expired retained output receipt is fail-closed",
)

# Each live lane already reserved a receipt slot. Store cannot evict another
# terminal fact after an engine ACK; that would make an older retry executable.
replace_exact(
    runtime,
    '''        purge_output_control_expired(&mut inner, now);
        enforce_output_control_receipt_capacity(&mut inner, &key, now);
        let last_used = next_output_control_sequence(&mut inner);
        inner.receipts.insert(
''',
    '''        purge_output_control_expired(&mut inner, now);
        inner.lanes.remove(&key);
        debug_assert!(inner.receipts.len() < MAX_TOTAL_RECEIPTS);
        debug_assert!(
            inner
                .receipts
                .keys()
                .filter(|candidate| {
                    candidate.principal == key.principal
                        && candidate.operation_id == key.operation_id
                })
                .count()
                < MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        );
        let last_used = next_output_control_sequence(&mut inner);
        inner.receipts.insert(
''',
    "store terminal only into its reserved slot",
)
replace_exact(
    runtime,
    '''        );
        inner.lanes.remove(&key);
        Ok(())
    }

    fn release_output_control_lane(&self, key: &OutputControlReceiptKey) {
''',
    '''        );
        Ok(())
    }

    fn release_output_control_lane(&self, key: &OutputControlReceiptKey) {
''',
    "terminal store no second lane removal",
)

# Expire tombstones first. If their bounded store is still full, keep the full
# expired receipt in-place and answer `receipt_expired`; retaining extra detail
# internally is safer than forgetting the request identity and re-executing it.
replace_exact(
    runtime,
    '''fn purge_output_control_expired(inner: &mut OutputControlPlaneInner, now: Instant) {
    let expired = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_output_control_tombstone(inner, key, now);
    }
    inner.tombstones.retain(|_, record| record.expires_at > now);
}

fn enforce_output_control_receipt_capacity(
    inner: &mut OutputControlPlaneInner,
    key: &OutputControlReceiptKey,
    now: Instant,
) {
    while inner.receipts.len() >= MAX_TOTAL_RECEIPTS && !inner.receipts.contains_key(key) {
        if !evict_oldest_output_control_receipt(inner, |_| true, now) {
            break;
        }
    }
    while inner
        .receipts
        .keys()
        .filter(|candidate| {
            candidate.principal == key.principal && candidate.operation_id == key.operation_id
        })
        .count()
        >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        && !inner.receipts.contains_key(key)
    {
        if !evict_oldest_output_control_receipt(
            inner,
            |candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            },
            now,
        ) {
            break;
        }
    }
}

fn evict_oldest_output_control_receipt(
    inner: &mut OutputControlPlaneInner,
    predicate: impl Fn(&OutputControlReceiptKey) -> bool,
    now: Instant,
) -> bool {
    let oldest = inner
        .receipts
        .iter()
        .filter(|(key, _)| predicate(key))
        .min_by_key(|(_, record)| record.last_used)
        .map(|(key, _)| key.clone());
    let Some(oldest) = oldest else {
        return false;
    };
    inner.receipts.remove(&oldest);
    inner.lanes.remove(&oldest);
    insert_output_control_tombstone(inner, oldest, now);
    true
}

fn insert_output_control_tombstone(
    inner: &mut OutputControlPlaneInner,
    key: OutputControlReceiptKey,
    now: Instant,
) {
    while inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES && !inner.tombstones.contains_key(&key) {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, record)| record.last_used)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_output_control_sequence(inner);
    inner.tombstones.insert(
        key,
        SafetyTombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
}
''',
    '''fn purge_output_control_expired(inner: &mut OutputControlPlaneInner, now: Instant) {
    inner.tombstones.retain(|_, record| record.expires_at > now);
    let expired = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired {
        if inner.tombstones.contains_key(&key) || inner.tombstones.len() < MAX_TOTAL_TOMBSTONES {
            inner.receipts.remove(&key);
            inner.lanes.remove(&key);
            let _ = insert_output_control_tombstone(inner, key, now);
        }
    }
}

fn insert_output_control_tombstone(
    inner: &mut OutputControlPlaneInner,
    key: OutputControlReceiptKey,
    now: Instant,
) -> bool {
    if !inner.tombstones.contains_key(&key) && inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES {
        return false;
    }
    let last_used = next_output_control_sequence(inner);
    inner.tombstones.insert(
        key,
        SafetyTombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
    true
}
''',
    "never evict output receipt/tombstone identity",
)

# Principal retirement must also refuse to forget an old request identity when
# tombstone storage is saturated. Keep a terminal record (marked expired) or a
# lane in memory rather than dropping the identity; the owner-incarnation key
# makes those retained records unreachable to the replacement renderer.
replace_exact(
    runtime,
    '''        for key in keys {
            output_control.receipts.remove(&key);
            output_control.lanes.remove(&key);
            insert_output_control_tombstone(&mut output_control, key, now);
        }
''',
    '''        for key in keys {
            if output_control.tombstones.contains_key(&key)
                || output_control.tombstones.len() < MAX_TOTAL_TOMBSTONES
            {
                output_control.receipts.remove(&key);
                output_control.lanes.remove(&key);
                let _ = insert_output_control_tombstone(&mut output_control, key, now);
            } else if let Some(record) = output_control.receipts.get_mut(&key) {
                record.expires_at = now;
            }
        }
''',
    "principal retirement preserves output request identity under capacity pressure",
)

# Focused deterministic capacity proof: no tombstone eviction and no new lane
# once every bounded terminal slot is reserved.
replace_exact(
    runtime,
    '''    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
''',
    '''    #[test]
    fn blackout_release_capacity_never_forgets_a_late_retry_identity() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-capacity", "main", 71);

        {
            let mut inner = state.output_control.lock().unwrap();
            for request_id in 1..=MAX_TOTAL_TOMBSTONES as u64 {
                let key = OutputControlReceiptKey {
                    principal: binding.principal.clone(),
                    window_label: binding.window_label.clone(),
                    owner_incarnation: binding.owner_incarnation,
                    operation_id: "old-operation".to_string(),
                    request_id,
                };
                assert!(insert_output_control_tombstone(&mut inner, key, now));
            }
            let oldest = OutputControlReceiptKey {
                principal: binding.principal.clone(),
                window_label: binding.window_label.clone(),
                owner_incarnation: binding.owner_incarnation,
                operation_id: "old-operation".to_string(),
                request_id: 1,
            };
            assert!(inner.tombstones.contains_key(&oldest));
            let overflow = OutputControlReceiptKey {
                request_id: (MAX_TOTAL_TOMBSTONES as u64) + 1,
                ..oldest.clone()
            };
            assert!(!insert_output_control_tombstone(&mut inner, overflow, now));
            assert!(inner.tombstones.contains_key(&oldest));
        }

        let shape = "a".repeat(64);
        for request_id in 1..=MAX_RECEIPTS_PER_PRINCIPAL_OPERATION as u64 {
            let key = OutputControlReceiptKey {
                principal: binding.principal.clone(),
                window_label: binding.window_label.clone(),
                owner_incarnation: binding.owner_incarnation,
                operation_id: "syndocal.output.blackout.release.v1".to_string(),
                request_id,
            };
            assert!(matches!(
                state.reserve_output_control_lane(&key, &shape, now),
                OutputControlLaneReservation::Lane(_)
            ));
        }
        let overflow = OutputControlReceiptKey {
            principal: binding.principal,
            window_label: binding.window_label,
            owner_incarnation: binding.owner_incarnation,
            operation_id: "syndocal.output.blackout.release.v1".to_string(),
            request_id: (MAX_RECEIPTS_PER_PRINCIPAL_OPERATION as u64) + 1,
        };
        assert!(matches!(
            state.reserve_output_control_lane(&overflow, &shape, now),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded)
        ));
    }

    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
''',
    "focused fail-closed output idempotency capacity test",
)
