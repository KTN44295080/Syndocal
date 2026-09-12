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

/** Only native-claimed requests enter here. Mutations still use the GUI transaction/CAS path. */
export async function executeAgentBridgeRequest(
  invoke: FrontendTauriInvoke,
  request: AgentBridgeRequest,
  effects?: AgentBridgeEffects,
) {
  let mutationStarted = false;
  try {
    if (!["fixtures.list", "fixtures.get", "fixtures.set_transform", "runtime.get", "output.set_video_blackout", "control_plane.get_capabilities", "recording.get_status", "control_plane.execute"].includes(request.method)) {
      return { ok: false, error: { code: "unknown_method", message: "Unsupported agent bridge operation." } };
    }
    const params = request.params;
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
    return { ok: false, error: {
      code: mutationStarted ? "mutation_not_confirmed" : "request_rejected",
      message: String(error).slice(0, 1024),
    } };
  }
}
