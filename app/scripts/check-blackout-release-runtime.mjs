import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [controllerSource, controlSource, panelSource, appSource, fullLockOverlaySource] = await Promise.all([
  readFile(new URL("../src/blackoutReleaseRuntimeController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/BlackoutReleaseControl.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/LightingRuntimeControlsPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),
]);

const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "blackoutReleaseRuntimeController.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);

const hash = (character) => character.repeat(64);
const opaqueId = "AAAAAAAAAAAAAAAAAAAAAA";
const fence = {
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: 3,
  project_revision: 4,
  project_checkpoint_hash: hash("a"),
  project_publication_generation: 5,
  output_epoch: 6,
  output_generation: 7,
  safety_blackout_epoch: 8,
  safety_blackout_generation: 9,
};
const fingerprint = hash("b");
const appliedFence = {
  ...fence,
  safety_blackout_generation: fence.safety_blackout_generation + 1,
};

const executeRequests = [];
const commands = [];
const replyLossController = runtime.createBlackoutReleaseRuntimeController({
  invoke: async (command, args) => {
    commands.push(command);
    switch (command) {
      case "query_output_control_authority_v1":
        assert.equal(args, undefined);
        return {
          operation_id: runtime.outputControlAuthorityQueryOperationId,
          fence,
        };
      case "prepare_output_consent_v1": {
        const request = args?.request;
        assert.deepEqual(request.expected_fence, fence);
        assert.deepEqual(request.action, { kind: "release_blackout" });
        return {
          operation_id: runtime.outputConsentPrepareOperationId,
          request_id: request.request_id,
          target_operation_id: runtime.blackoutReleaseOperationId,
          challenge_id: opaqueId,
          consent_token: opaqueId,
          display_code: "123456",
          argument_fingerprint: fingerprint,
          expires_at_unix_ms: Date.now() + 15_000,
        };
      }
      case "query_output_consent_status_v1": {
        const request = args?.request;
        assert.equal(request.challenge_id, opaqueId);
        return {
          operation_id: runtime.outputConsentStatusQueryOperationId,
          request_id: request.request_id,
          challenge_id: opaqueId,
          state: "ready",
          expires_at_unix_ms: Date.now() + 10_000,
        };
      }
      case "execute_output_control_v1": {
        const request = args?.request;
        executeRequests.push(request);
        assert.deepEqual(request.expected_fence, fence);
        assert.equal(request.consent_token, opaqueId);
        assert.deepEqual(request.action, { kind: "release_blackout" });
        if (executeRequests.length === 1) {
          throw new Error("synthetic lost IPC reply after engine publication");
        }
        return {
          type: "receipt",
          receipt: {
            operation_id: runtime.blackoutReleaseOperationId,
            request_id: request.request_id,
            shape_sha256: hash("c"),
            argument_fingerprint: fingerprint,
            audit_sequence: 1,
            fence_before: fence,
            fence_after: appliedFence,
            outcome: "applied",
          },
        };
      }
      default:
        throw new Error(`unexpected command ${command}`);
    }
  },
});

const prepared = await replyLossController.prepareRelease();
assert.equal(await replyLossController.consentStatus(prepared), "ready");
const applied = await replyLossController.release(prepared);
assert.equal(applied.outcome, "applied");
assert.equal(executeRequests.length, 2);
assert.strictEqual(
  executeRequests[1],
  executeRequests[0],
  "reply-loss recovery must reuse the exact execution request object",
);
assert.deepEqual(commands, [
  "query_output_control_authority_v1",
  "prepare_output_consent_v1",
  "query_output_consent_status_v1",
  "execute_output_control_v1",
  "execute_output_control_v1",
]);

let mismatchedReceiptCalls = 0;
const mismatchedReceipt = runtime.createBlackoutReleaseRuntimeController({
  invoke: async (command, args) => {
    assert.equal(command, "execute_output_control_v1");
    mismatchedReceiptCalls += 1;
    return {
      type: "receipt",
      receipt: {
        operation_id: runtime.blackoutReleaseOperationId,
        request_id: args.request.request_id,
        shape_sha256: hash("c"),
        argument_fingerprint: hash("d"),
        audit_sequence: 2,
        fence_before: fence,
        fence_after: appliedFence,
        outcome: "applied",
      },
    };
  },
});
await assert.rejects(mismatchedReceipt.release(prepared), /invalid terminal receipt/);
assert.equal(mismatchedReceiptCalls, 1);

const nonCanonicalChallenge = runtime.createBlackoutReleaseRuntimeController({
  invoke: async (command, args) => {
    if (command === "query_output_control_authority_v1") {
      return { operation_id: runtime.outputControlAuthorityQueryOperationId, fence };
    }
    assert.equal(command, "prepare_output_consent_v1");
    return {
      operation_id: runtime.outputConsentPrepareOperationId,
      request_id: args.request.request_id,
      target_operation_id: runtime.blackoutReleaseOperationId,
      challenge_id: "AAAAAAAAAAAAAAAAAAAAAB",
      consent_token: opaqueId,
      display_code: "123456",
      argument_fingerprint: fingerprint,
      expires_at_unix_ms: Date.now() + 15_000,
    };
  },
});
await assert.rejects(nonCanonicalChallenge.prepareRelease(), /invalid prepared consent challenge/);

for (const command of runtime.blackoutReleaseInvokeCommands) {
  assert.match(controlSource, new RegExp(`"${command}"`));
}
assert.doesNotMatch(controlSource, /set_blackout/);
assert.match(controlSource, /physical keyboard connected to this machine/i);
assert.match(controlSource, /releaseRuntime\.consentStatus\(consent\)/);
assert.match(controlSource, /releaseRuntime\.release\(consent\)/);
assert.match(panelSource, /<BlackoutReleaseControl\s+onReleased=\{props\.onBlackoutReleased\}\s*\/>/);
assert.doesNotMatch(panelSource, /onBlackout\(false\)/);
assert.doesNotMatch(appSource, /invoke\("set_blackout", \{ enabled: false \}\)/);
assert.match(appSource, /onBlackoutReleased=\{refreshSnapshot\}/);
assert.match(fullLockOverlaySource, /BLACKOUT RELEASE LOCKED/);
assert.doesNotMatch(fullLockOverlaySource, /onSetBlackout\(false\)/);

console.log("blackout release runtime contract: PASS");
