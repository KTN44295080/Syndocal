import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  OutputOwnershipStatus,
  PatchedFixtureSummary,
  ProjectAuthorityBundle,
  VideoOutputSummary,
} from "./types";
import { fixtureTransformMatchesExpectation } from "./fixtureTransformConfirmation";
import {
  executeAgentBridgeVideoBlackout,
  type AgentBridgeEffects,
} from "./agentBridgeBlackout";
import {
  executeAgentBridgeCanonicalOperation,
  executeAgentBridgeControlPlane,
  CANONICAL_TAURI_COMMANDS,
  canonicalOperationIsMutation,
  NATIVE_TIMELINE_MUTATIONS,
} from "./agentBridgeControlPlane";
import { executeAgentBridgeRecordingStatus } from "./agentBridgeRecording";

export type { AgentBridgeEffects } from "./agentBridgeBlackout";

export interface AgentBridgeRequest {
  rendererGeneration: number;
  requestId: string;
  method: string;
  params: Record<string, unknown>;
  principalId: string;
  principalIncarnation: number;
}

const projectToken = (bundle: ProjectAuthorityBundle) => ({
  project_epoch: bundle.project_epoch,
  project_revision: bundle.project_revision,
  checkpoint_hash: bundle.checkpoint_hash,
});
const fixtureView = (fixture: PatchedFixtureSummary) => ({
  id: fixture.id, label: fixture.label, position: fixture.position, rotation: fixture.rotation,
});
const videoOutputView = (output: VideoOutputSummary) => ({
  id: output.id,
  name: output.label,
  enabled: output.enabled,
  composition_id: output.composition_id,
  dimensions: { width: output.width, height: output.height },
});
const ownershipStatusView = (status: OutputOwnershipStatus) => ({
  role: status.role,
  effective_role: status.effective_role,
  desired_role: status.desired_role,
  persisted_role: status.persisted_role,
  state: status.state,
  generation: status.generation,
  epoch: status.epoch,
  lighting_allowed: status.lighting_allowed,
  video_allowed: status.video_allowed,
  lighting_reason: status.lighting_reason,
  video_reason: status.video_reason,
  error: status.error,
});

// Tauri serializes canonical QueryError as an object, not an Error instance.
// Preserve its bounded wire fields without forwarding arbitrary exception data.
const nativeQueryError = (error: unknown) => {
  if (!error || typeof error !== "object" || error instanceof Error) return undefined;
  const value = error as Record<string, unknown>;
  if (Object.keys(value).length !== 4
    || !["code", "message", "retryable", "resnapshot_required"].every(key => Object.hasOwn(value, key))
    || typeof value.code !== "string" || !/^[a-z_]{1,64}$/.test(value.code)
    || typeof value.message !== "string" || value.message.length > 1024
    || typeof value.retryable !== "boolean" || typeof value.resnapshot_required !== "boolean") return undefined;
  return { code: value.code, message: value.message,
    retryable: value.retryable, resnapshot_required: value.resnapshot_required };
};

const runtimeAuthorityOperationIds = new Set([
  "syndocal.query.runtime.timeline.transport.authority.v1",
  "syndocal.query.runtime.timeline.loop.authority.v1",
  "syndocal.query.runtime.timeline.follow.abort.authority.v1",
]);
const runtimeAuthorityErrorCodes = new Set([
  "invalid_request", "forbidden", "stale_fence", "conflict", "busy",
  "overloaded", "publication_failed", "internal",
]);
// RuntimeCommandErrorV1 has a different wire from canonical QueryError.
// Recognize only its exact fixed-code object on the three authority reads;
// arbitrary objects and uncertain mutation failures remain unclassified.
const nativeRuntimeAuthorityError = (error: unknown) => {
  if (!error || typeof error !== "object" || error instanceof Error) return undefined;
  const value = error as Record<string, unknown>;
  if (Object.keys(value).length !== 1 || !Object.hasOwn(value, "code")
    || typeof value.code !== "string" || !runtimeAuthorityErrorCodes.has(value.code)) return undefined;
  return { code: value.code };
};

/** Only native-claimed requests enter here. Mutations still use the GUI transaction/CAS path. */
export async function executeAgentBridgeRequest(
  invoke: FrontendTauriInvoke,
  request: AgentBridgeRequest,
  effects?: AgentBridgeEffects,
) {
  let mutationStarted = false;
  try {
    if (!["fixtures.list", "fixtures.get", "fixtures.set_transform", "runtime.get", "output.set_video_blackout", "control_plane.get_capabilities", "recording.get_status", "control_plane.execute", "diagnostics.export"].includes(request.method)) {
      return { ok: false, error: { code: "unknown_method", message: "Unsupported agent bridge operation." } };
    }
    const params = request.params;
    if (request.method === "diagnostics.export"
      || (request.method === "control_plane.execute" && typeof params.operationId === "string"
        && (params.operationId.startsWith("syndocal.output.")
          || NATIVE_TIMELINE_MUTATIONS.has(params.operationId)
          || params.operationId === "syndocal.project.new.v1"
          || params.operationId === "syndocal.project.open.v1"
          || params.operationId === "syndocal.project.backup.restore.v1"
          || params.operationId === "syndocal.project.backup.delete.v1")
        && canonicalOperationIsMutation(params.operationId)
        || (request.method === "control_plane.execute" && typeof params.operationId === "string"
          && ["syndocal.query.project.backup.delete.journal.v1","syndocal.query.project.backup.delete.journal.status.v1","syndocal.project.backup.delete.journal.manage.v1","syndocal.query.project.backup.delete.status.v1","syndocal.query.project.backup.list.v1","syndocal.query.project.backup.inspect.v1","syndocal.project.backup.create.v1","syndocal.query.project.backup.authority.v1","syndocal.project.save.v1","syndocal.project.save_as.v1","syndocal.project.template.save.v1","syndocal.query.project.file.authority.v1","syndocal.query.project.file.status.v1","syndocal.project.file.acknowledge.v1"].includes(params.operationId)))) {
      if (request.method === "control_plane.execute" && !Object.hasOwn(CANONICAL_TAURI_COMMANDS, params.operationId as string)) {
        throw new Error("Canonical operation is not executable through the reviewed adapter set.");
      }
      mutationStarted = request.method === "diagnostics.export" || canonicalOperationIsMutation(params.operationId as string);
      return await invoke("agent_bridge_execute_native_v1", {
        rendererGeneration: request.rendererGeneration,
        requestId: request.requestId,
      });
    }
    if (request.method === "control_plane.execute") {
      return await executeAgentBridgeCanonicalOperation(invoke, params, () => { mutationStarted = true; });
    }
    if (request.method === "control_plane.get_capabilities") {
      if (Object.keys(params).length !== 0) throw new Error("Control-plane capability discovery takes no parameters.");
      return await executeAgentBridgeControlPlane(invoke);
    }
    if (request.method === "recording.get_status") {
      if (Object.keys(params).length !== 0) throw new Error("Recording status takes no parameters.");
      return await executeAgentBridgeRecordingStatus(invoke);
    }
    if (request.method === "output.set_video_blackout") {
      return await executeAgentBridgeVideoBlackout(
        invoke,
        params,
        effects,
        () => { mutationStarted = true; },
      );
    }
    const expected = params.expectedProject as ReturnType<typeof projectToken> | undefined;
    const setTransform = request.method === "fixtures.set_transform";
    if (setTransform && !expected) throw new Error("Expected project token is required.");
    const before = await invoke<ProjectAuthorityBundle>("get_project_authority_bundle", setTransform ? {
      expectedEpoch: expected!.project_epoch,
      expectedRevision: expected!.project_revision,
      expectedCheckpointHash: expected!.checkpoint_hash,
    } : {});
    if (request.method === "runtime.get") {
      // The authority bundle and ownership status are deliberately separate
      // observations. The bundle is atomic; this second read is not folded
      // into that claim and a failure rejects the whole diagnostic request.
      const ownership = await invoke<OutputOwnershipStatus>("get_output_ownership_status");
      const snapshot = before.snapshot;
      const outputs = snapshot.video.outputs;
      return {
        ok: true,
        project: projectToken(before),
        blackout: snapshot.blackout,
        authored_blackout: snapshot.authored_blackout,
        safety_blackout_engaged: snapshot.safety_blackout_engaged,
        video: {
          blackout: snapshot.video.blackout,
          // Public bundles deliberately omit the backend-only authored_video
          // image; this persisted field is the authoritative video target bit.
          authored_blackout: snapshot.video.blackout,
          outputs: outputs.slice(0, 64).map(videoOutputView),
          total: outputs.length,
          truncated: outputs.length > 64,
        },
        timeline_runtime: before.timeline_runtime,
        timeline: {
          id: snapshot.timeline.id,
          playing: snapshot.timeline.playing,
          position_ms: snapshot.timeline.position_ms,
          duration_ms: snapshot.timeline.duration_ms,
        },
        observations: {
          output_ownership_status: ownershipStatusView(ownership),
        },
      };
    }
    if (request.method === "fixtures.list") {
      const fixtures = before.snapshot.fixtures;
      return { ok: true, project: projectToken(before), fixtures: fixtures.slice(0, 256).map(fixtureView),
        total: fixtures.length, truncated: fixtures.length > 256 };
    }
    const fixture = before.snapshot.fixtures.find(value => value.id === params.fixtureId);
    if (!fixture) throw new Error("Fixture does not exist in the current project.");
    if (!setTransform) return { ok: true, project: projectToken(before), fixture: fixtureView(fixture) };
    mutationStarted = true;
    await invoke("set_fixture_transform", {
      fixtureId: params.fixtureId, position: params.position, rotation: params.rotation,
      __expectedProjectEpoch: expected!.project_epoch,
      __expectedProjectRevision: expected!.project_revision,
      __expectedCheckpointHash: expected!.checkpoint_hash,
    });
    const after = await invoke<ProjectAuthorityBundle>("get_project_authority_bundle", {});
    const actual = after.snapshot.fixtures.find(value => value.id === params.fixtureId);
    const verified = after.project_epoch === before.project_epoch && fixtureTransformMatchesExpectation(actual, {
      position: params.position as PatchedFixtureSummary["position"],
      rotation: params.rotation as PatchedFixtureSummary["rotation"],
    });
    return verified && actual
      ? { ok: true, project: projectToken(after), fixture: fixtureView(actual), verification: "committed_project_state" }
      : { ok: false, error: { code: "verification_failed", message: "Operation returned, but the requested transform was not confirmed. Read current state before any new operation." } };
  } catch (error) {
    const query = mutationStarted ? undefined : nativeQueryError(error);
    const runtime = !mutationStarted && request.method === "control_plane.execute"
      && typeof request.params.operationId === "string"
      && runtimeAuthorityOperationIds.has(request.params.operationId)
      ? nativeRuntimeAuthorityError(error) : undefined;
    return { ok: false, error: {
      code: mutationStarted ? "mutation_not_confirmed" : "request_rejected",
      message: query?.message ?? (runtime ? `Native runtime authority read rejected: ${runtime.code}.`
        : error && typeof error === "object" && !(error instanceof Error)
        ? "Native operation failed with an unrecognized error response." : String(error).slice(0, 1024)),
      ...(query ? { native_query: query } : {}),
      ...(runtime ? { native_runtime: runtime } : {}),
    } };
  }
}
