from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
runtime = Path("app/src-tauri/src/control_plane_runtime.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
reply_loss_test = Path(".github/scripts/patch_blackout_release_reply_loss_test.py")

replace_exact(
    protocol,
    '''    InvalidRequest,
    Forbidden,
''',
    '''    InvalidRequest,
    ReceiptExpired,
    Forbidden,
''',
    "output-control receipt-expired error code",
)

runtime_text = runtime.read_text(encoding="utf-8")
old = '''        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::InvalidRequest,
            );
        }
'''
new = '''        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::ReceiptExpired,
            );
        }
'''
if runtime_text.count(old) != 1:
    raise RuntimeError(
        f"output-control reserve tombstone: expected exactly one match, found {runtime_text.count(old)}"
    )
runtime_text = runtime_text.replace(old, new, 1)
old = '''        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::InvalidRequest,
            ));
        }
'''
new = '''        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::ReceiptExpired,
            ));
        }
'''
if runtime_text.count(old) != 1:
    raise RuntimeError(
        f"output-control recheck tombstone: expected exactly one match, found {runtime_text.count(old)}"
    )
runtime.write_text(runtime_text.replace(old, new, 1), encoding="utf-8")

replace_exact(
    controller,
    '''  | "invalid_request"
  | "forbidden"
''',
    '''  | "invalid_request"
  | "receipt_expired"
  | "forbidden"
''',
    "renderer receipt-expired error type",
)
replace_exact(
    controller,
    '''  "invalid_request",
  "forbidden",
''',
    '''  "invalid_request",
  "receipt_expired",
  "forbidden",
''',
    "renderer receipt-expired validation set",
)

# This script edits the staged test generator because the actual test is added
# later in the same workflow. Keep its expected tombstone outcome aligned with
# the protocol/runtime contract generated above.
replace_exact(
    reply_loss_test,
    '''            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));
    }
''',
    '''            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::ReceiptExpired)
        ));
    }
''',
    "R4 reply-loss tombstone expected outcome",
)
