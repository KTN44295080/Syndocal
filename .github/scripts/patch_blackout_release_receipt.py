from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    old_count = text.count(old)
    new_count = text.count(new)
    if old_count != 1 or new_count != 0:
        raise RuntimeError(
            f"{label}: expected old=1/new=0, found old={old_count}/new={new_count}"
        )
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
checker = Path("app/scripts/check-blackout-release-runtime.mjs")

replace_exact(
    protocol,
    '''        match self.outcome {
            OutputControlReceiptOutcomeV1::NoOp if self.fence_before != self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            OutputControlReceiptOutcomeV1::Applied if self.fence_before == self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            _ => Ok(()),
        }
''',
    '''        match self.outcome {
            OutputControlReceiptOutcomeV1::NoOp if self.fence_before != self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            OutputControlReceiptOutcomeV1::Applied
                if self.operation_id == OUTPUT_BLACKOUT_RELEASE_OPERATION_ID =>
            {
                validate_blackout_release_applied_fence_transition(
                    &self.fence_before,
                    &self.fence_after,
                )
            }
            OutputControlReceiptOutcomeV1::Applied if self.fence_before == self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            _ => Ok(()),
        }
''',
    "protocol receipt outcome validation",
)

replace_exact(
    protocol,
    '''fn validate_output_request_id(request_id: u64) -> Result<(), OutputControlValidationErrorV1> {
''',
    '''fn validate_blackout_release_applied_fence_transition(
    before: &OutputControlFenceV1,
    after: &OutputControlFenceV1,
) -> Result<(), OutputControlValidationErrorV1> {
    let Some((safety_blackout_epoch, safety_blackout_generation)) =
        next_timeline_transport_authority(
            before.safety_blackout_epoch,
            before.safety_blackout_generation,
        )
    else {
        return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
    };
    if after.process_incarnation == before.process_incarnation
        && after.session_incarnation == before.session_incarnation
        && after.project_epoch == before.project_epoch
        && after.project_revision == before.project_revision
        && after.project_checkpoint_hash == before.project_checkpoint_hash
        && after.project_publication_generation == before.project_publication_generation
        && after.output_epoch == before.output_epoch
        && after.output_generation == before.output_generation
        && after.safety_blackout_epoch == safety_blackout_epoch
        && after.safety_blackout_generation == safety_blackout_generation
    {
        Ok(())
    } else {
        Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
    }
}

fn validate_output_request_id(request_id: u64) -> Result<(), OutputControlValidationErrorV1> {
''',
    "protocol exact blackout release transition helper",
)

replace_exact(
    protocol,
    '''        let mut invalid_noop = match response {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        invalid_noop.outcome = OutputControlReceiptOutcomeV1::NoOp;
        assert!(serde_json::to_value(invalid_noop).is_err());
''',
    '''        let mut invalid_noop = match response.clone() {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        invalid_noop.outcome = OutputControlReceiptOutcomeV1::NoOp;
        assert!(serde_json::to_value(invalid_noop).is_err());

        let mut output_drift = match response.clone() {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        output_drift.fence_after = output_drift.fence_before.clone();
        output_drift.fence_after.output_generation += 1;
        assert!(serde_json::to_value(output_drift).is_err());

        let mut skipped_safety_authority = match response {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        skipped_safety_authority.fence_after.safety_blackout_generation += 1;
        assert!(serde_json::to_value(skipped_safety_authority).is_err());
''',
    "protocol forged release receipt tests",
)

replace_exact(
    controller,
    '''const sameFence = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;
''',
    '''const sameFence = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;

const nextAuthorityPair = (epoch: number, generation: number): readonly [number, number] | null => {
  if (generation < Number.MAX_SAFE_INTEGER) return [epoch, generation + 1] as const;
  if (epoch < Number.MAX_SAFE_INTEGER) return [epoch + 1, 1] as const;
  return null;
};

const isExactBlackoutReleaseAppliedTransition = (
  before: OutputControlFence,
  after: OutputControlFence,
): boolean => {
  const next = nextAuthorityPair(
    before.safety_blackout_epoch,
    before.safety_blackout_generation,
  );
  return next !== null
    && after.process_incarnation === before.process_incarnation
    && after.session_incarnation === before.session_incarnation
    && after.project_epoch === before.project_epoch
    && after.project_revision === before.project_revision
    && after.project_checkpoint_hash === before.project_checkpoint_hash
    && after.project_publication_generation === before.project_publication_generation
    && after.output_epoch === before.output_epoch
    && after.output_generation === before.output_generation
    && after.safety_blackout_epoch === next[0]
    && after.safety_blackout_generation === next[1];
};
''',
    "renderer exact blackout release transition helper",
)

replace_exact(
    controller,
    '''  if ((receipt.outcome === "no_op" && !sameFence(receipt.fence_before, receipt.fence_after))
    || (receipt.outcome === "applied" && sameFence(receipt.fence_before, receipt.fence_after))) {
    throw new BlackoutReleaseProtocolError("receipt fence transition");
  }
''',
    '''  if ((receipt.outcome === "no_op" && !sameFence(receipt.fence_before, receipt.fence_after))
    || (receipt.outcome === "applied"
      && !isExactBlackoutReleaseAppliedTransition(receipt.fence_before, receipt.fence_after))) {
    throw new BlackoutReleaseProtocolError("receipt fence transition");
  }
''',
    "renderer receipt transition validation",
)

replace_exact(
    checker,
    '''await assert.rejects(mismatchedReceipt.release(prepared), /invalid terminal receipt/);
assert.equal(mismatchedReceiptCalls, 1);

const nonCanonicalChallenge = runtime.createBlackoutReleaseRuntimeController({
''',
    '''await assert.rejects(mismatchedReceipt.release(prepared), /invalid terminal receipt/);
assert.equal(mismatchedReceiptCalls, 1);

let forgedTransitionCalls = 0;
const forgedTransition = runtime.createBlackoutReleaseRuntimeController({
  invoke: async (command, args) => {
    assert.equal(command, "execute_output_control_v1");
    forgedTransitionCalls += 1;
    return {
      type: "receipt",
      receipt: {
        operation_id: runtime.blackoutReleaseOperationId,
        request_id: args.request.request_id,
        shape_sha256: hash("c"),
        argument_fingerprint: fingerprint,
        audit_sequence: 3,
        fence_before: fence,
        fence_after: {
          ...fence,
          output_generation: fence.output_generation + 1,
        },
        outcome: "applied",
      },
    };
  },
});
await assert.rejects(forgedTransition.release(prepared), /invalid receipt fence transition/);
assert.equal(forgedTransitionCalls, 1);

const nonCanonicalChallenge = runtime.createBlackoutReleaseRuntimeController({
''',
    "renderer forged transition harness",
)
