import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { PatchedFixtureSummary, ProjectAuthorityBundle } from "./types";
import { fixtureTransformMatchesExpectation } from "./fixtureTransformConfirmation";

export interface AgentBridgeRequest {
  rendererGeneration: number;
  requestId: string;
  method: string;
  params: Record<string, unknown>;
}

const projectToken = (bundle: ProjectAuthorityBundle) => ({
  project_epoch: bundle.project_epoch,
  project_revision: bundle.project_revision,
  checkpoint_hash: bundle.checkpoint_hash,
});
const fixtureView = (fixture: PatchedFixtureSummary) => ({
  id: fixture.id, label: fixture.label, position: fixture.position, rotation: fixture.rotation,
});

/** Only native-claimed requests enter here. Mutations still use the GUI transaction/CAS path. */
export async function executeAgentBridgeRequest(invoke: FrontendTauriInvoke, request: AgentBridgeRequest) {
  let mutationStarted = false;
  try {
    if (!["fixtures.list", "fixtures.get", "fixtures.set_transform"].includes(request.method)) {
      return { ok: false, error: { code: "unknown_method", message: "Unsupported fixture operation." } };
    }
    const params = request.params;
    const expected = params.expectedProject as ReturnType<typeof projectToken> | undefined;
    const setTransform = request.method === "fixtures.set_transform";
    if (setTransform && !expected) throw new Error("Expected project token is required.");
    const before = await invoke<ProjectAuthorityBundle>("get_project_authority_bundle", setTransform ? {
      expectedEpoch: expected!.project_epoch,
      expectedRevision: expected!.project_revision,
      expectedCheckpointHash: expected!.checkpoint_hash,
    } : {});
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
