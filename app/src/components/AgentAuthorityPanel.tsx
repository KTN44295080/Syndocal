import { For, Show, createSignal, onMount } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";

type Capability = "read" | "runtime" | "live" | "authored" | "output" | "file" | "safety_blackout_engage";
type Risk = "R0" | "R1" | "R2" | "R3" | "R4" | "R5" | "S0";

type AgentGrant = {
  adapter: string;
  capability: Capability;
  operation_id: string;
  project_id: string | null;
};

type Principal = {
  principalId: string;
  principalIncarnation: number;
  mode: "safe" | "promoted";
  revoked: boolean;
  grants: AgentGrant[];
};

type AuthorityStatus = {
  killSwitchActive: boolean;
  activeSessions: number;
  principals: Principal[];
  audit: AuditRecord[];
};

type AuditRecord = {
  sequence: number;
  event: string;
  principalId: string | null;
  principalIncarnation: number | null;
  operationId: string | null;
  outcome: string;
};

type PairingChallenge = {
  challengeId: string;
  principalId: string;
  challenge: string;
  expiresInMs: number;
};

type PairingApproval = {
  principalId: string;
  principalIncarnation: number;
  credential: string;
};

type Props = {
  invokeCommand: FrontendTauriInvoke;
};

const capabilities: Capability[] = ["read", "runtime", "live", "authored", "output", "file", "safety_blackout_engage"];
const risks: Risk[] = ["R0", "R1", "R2", "R3", "R4", "R5", "S0"];

const parseInteger = (value: string, fallback = 0) => {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : fallback;
};

const digestBytes = (value: string): number[] | null => {
  if (!/^[0-9a-f]{64}$/u.test(value)) return null;
  return Array.from({ length: 32 }, (_, index) => Number.parseInt(value.slice(index * 2, index * 2 + 2), 16));
};

export function AgentAuthorityPanel(props: Props) {
  const [status, setStatus] = createSignal<AuthorityStatus | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [message, setMessage] = createSignal<string | null>(null);
  const [pairPrincipal, setPairPrincipal] = createSignal("show-operator");
  const [challenge, setChallenge] = createSignal<PairingChallenge | null>(null);
  const [challengeResponse, setChallengeResponse] = createSignal("");
  const [approval, setApproval] = createSignal<PairingApproval | null>(null);
  const [selectedPrincipal, setSelectedPrincipal] = createSignal("");
  const [selectedIncarnation, setSelectedIncarnation] = createSignal(0);
  const [grantOperation, setGrantOperation] = createSignal("");
  const [grantCapability, setGrantCapability] = createSignal<Capability>("read");
  const [grantProject, setGrantProject] = createSignal("");
  const [consentId, setConsentId] = createSignal("");
  const [consentPrincipal, setConsentPrincipal] = createSignal("");
  const [consentIncarnation, setConsentIncarnation] = createSignal("1");
  const [consentOperation, setConsentOperation] = createSignal("");
  const [consentCapability, setConsentCapability] = createSignal<Capability>("output");
  const [consentRisk, setConsentRisk] = createSignal<Risk>("R4");
  const [consentDigest, setConsentDigest] = createSignal("0".repeat(64));
  const [consentOwner, setConsentOwner] = createSignal("1");
  const [consentProjectGeneration, setConsentProjectGeneration] = createSignal("0");
  const [consentOutputGeneration, setConsentOutputGeneration] = createSignal("0");

  const run = async <T,>(action: () => Promise<T>, success: (value: T) => void = () => {}) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      success(await action());
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const refresh = () => void run(
    () => props.invokeCommand<AuthorityStatus>("agent_authority_status_v1"),
    (next) => {
      setStatus(next);
      if (!selectedPrincipal() && next.principals[0]) {
        setSelectedPrincipal(next.principals[0].principalId);
        setSelectedIncarnation(next.principals[0].principalIncarnation);
      }
    },
  );

  onMount(refresh);

  const beginPairing = () => void run(
    () => props.invokeCommand<PairingChallenge>("agent_authority_begin_pairing_v1", { principalId: pairPrincipal() }),
    (next) => {
      setChallenge(next);
      setChallengeResponse("");
      setApproval(null);
      setMessage(`Pairing challenge created; it expires in ${next.expiresInMs} ms.`);
    },
  );

  const approvePairing = () => {
    const current = challenge();
    if (!current) return;
    void run(
      () => props.invokeCommand<PairingApproval>("agent_authority_approve_pairing_v1", { challengeId: current.challengeId, challenge: challengeResponse() }),
      (next) => {
        setApproval(next);
        setSelectedPrincipal(next.principalId);
        setSelectedIncarnation(next.principalIncarnation);
        setChallenge(null);
        setChallengeResponse("");
        setMessage("Principal paired in safe mode. Store the credential in the external client's protected credential file; this screen does not persist it.");
        refresh();
      },
    );
  };

  const promote = (principal: Principal) => void run(
    () => props.invokeCommand("agent_authority_promote_v1", { principalId: principal.principalId, principalIncarnation: principal.principalIncarnation }),
    () => { setMessage(`${principal.principalId} promoted; operation grants remain explicit.`); refresh(); },
  );

  const grant = () => {
    const principalId = selectedPrincipal();
    const operationId = grantOperation().trim();
    if (!principalId || !selectedIncarnation() || !operationId) {
      setError("Select a principal and enter an exact canonical operation ID.");
      return;
    }
    void run(
      () => props.invokeCommand("agent_authority_grant_v1", {
        principalId,
        principalIncarnation: selectedIncarnation(),
        grant: { adapter: "external_mcp", capability: grantCapability(), operation_id: operationId, project_id: grantProject().trim() || null },
      }),
      () => { setMessage("Exact ExternalMcp grant installed."); setGrantOperation(""); refresh(); },
    );
  };

  const revoke = (principal: Principal) => void run(
    () => props.invokeCommand("agent_authority_revoke_v1", { principalId: principal.principalId, principalIncarnation: principal.principalIncarnation }),
    () => { setMessage(`${principal.principalId} revoked and its grants removed.`); refresh(); },
  );

  const consentContext = () => {
    const fingerprint = digestBytes(consentDigest());
    if (!fingerprint) return null;
    return {
      principal: consentPrincipal(),
      principal_incarnation: parseInteger(consentIncarnation()),
      adapter: "local_tauri_window",
      operation_id: consentOperation().trim(),
      capability: consentCapability(),
      risk: consentRisk(),
      owner_incarnation: parseInteger(consentOwner()),
      canonical_arguments_fingerprint: fingerprint,
      project_id: null,
      project_generation: parseInteger(consentProjectGeneration()),
      output_generation: parseInteger(consentOutputGeneration()),
    };
  };

  const prepareConsent = () => {
    const context = consentContext();
    if (!context || !consentId().trim() || !consentPrincipal().trim() || !context.operation_id) {
      setError("Consent requires an ID, principal, exact operation, and a 64-character argument fingerprint.");
      return;
    }
    void run(
      () => props.invokeCommand("agent_authority_prepare_consent_v1", { consentId: consentId().trim(), context, nowMs: Date.now(), ttlMs: 15_000 }),
      () => setMessage("Single-use consent prepared for the exact principal, operation, arguments, generations, and 15-second window."),
    );
  };

  const authorizeConsent = () => {
    const context = consentContext();
    if (!context || !consentId().trim()) {
      setError("Prepare the exact consent context before authorization.");
      return;
    }
    void run(
      () => props.invokeCommand("agent_authority_authorize_with_consent_v1", { consentId: consentId().trim(), context, nowMs: Date.now() }),
      () => { setMessage("Prepared consent consumed once for this exact context."); setConsentId(""); },
    );
  };

  const killSwitch = () => {
    if (!window.confirm("Revoke every external principal and close all authority grants?")) return;
    void run(
      () => props.invokeCommand("agent_authority_kill_switch_v1"),
      () => { setMessage("Kill switch active: external principals are revoked and new pairing is blocked."); refresh(); },
    );
  };

  return (
    <aside class="panel setup setupPanel agentAuthorityPanel" data-agent-authority-panel tabIndex={-1}>
      <header class="panelHeader">
        <div><span class="panelEyebrow">SECURITY</span><h2>AI Access</h2></div>
        <button type="button" onClick={refresh} disabled={busy()} aria-label="Refresh AI authority status">Refresh</button>
      </header>
      <Show when={error()}><p class="errorMessage" role="alert">{error()}</p></Show>
      <Show when={message()}><p class="successMessage" role="status">{message()}</p></Show>
      <p class="ioDisclosureDescription">Pairing, grants, consent, and revocation stay in the trusted desktop. External clients never approve their own high-risk operations.</p>

      <section class="agentAuthoritySection" aria-labelledby="agent-pairing-heading">
        <h3 id="agent-pairing-heading">Pair principal</h3>
        <div class="agentAuthorityForm">
          <label>Principal ID<input value={pairPrincipal()} onInput={(event) => setPairPrincipal(event.currentTarget.value)} maxLength={128} /></label>
          <button type="button" onClick={beginPairing} disabled={busy() || !pairPrincipal().trim()}>Create challenge</button>
        </div>
        <Show when={challenge()}>
          {(current) => <div class="agentAuthorityChallenge" role="status">
            <strong>Enter this challenge in the trusted pairing flow</strong>
            <code data-agent-pairing-challenge>{current().challenge}</code>
            <label>Challenge response<input value={challengeResponse()} onInput={(event) => setChallengeResponse(event.currentTarget.value)} /></label>
            <button type="button" onClick={approvePairing} disabled={busy() || !challengeResponse()}>Approve pairing</button>
          </div>}
        </Show>
        <Show when={approval()}>
          {(current) => <div class="agentAuthorityCredential" role="status">
            <strong>Credential returned once</strong>
            <code data-agent-pairing-credential>{current().credential}</code>
            <small>Do not paste this value into project files, URLs, environment variables, or logs. This panel does not save it.</small>
          </div>}
        </Show>
      </section>

      <section class="agentAuthoritySection" aria-labelledby="agent-sessions-heading">
        <div class="agentAuthoritySectionHeading"><h3 id="agent-sessions-heading">Active principals</h3><span class={status()?.killSwitchActive ? "dangerStatus" : "safeStatus"}>{status()?.killSwitchActive ? "KILL SWITCH ACTIVE" : "External control fenced"}</span></div>
        <p class="ioDisclosureDescription">Live sidecar connections: <strong class="tabularNums">{status()?.activeSessions ?? 0} / 8</strong></p>
        <Show when={status()?.principals.length} fallback={<p class="emptyState">No paired external principals.</p>}>
          <div class="agentAuthorityPrincipalList">
            <For each={status()?.principals ?? []}>{(principal) => <article class="agentAuthorityPrincipal" data-agent-principal={principal.principalId}>
              <div><strong>{principal.principalId}</strong><span>incarnation {principal.principalIncarnation} · {principal.revoked ? "revoked" : principal.mode}</span></div>
              <span>{principal.grants.length} exact grant{principal.grants.length === 1 ? "" : "s"}</span>
              <div class="agentAuthorityActions">
                <button type="button" onClick={() => { setSelectedPrincipal(principal.principalId); setSelectedIncarnation(principal.principalIncarnation); }} disabled={busy()}>Select</button>
                <Show when={!principal.revoked && principal.mode === "safe"}><button type="button" onClick={() => promote(principal)} disabled={busy()}>Promote</button></Show>
                <Show when={!principal.revoked}><button type="button" class="danger" onClick={() => revoke(principal)} disabled={busy()}>Revoke</button></Show>
              </div>
            </article>}</For>
          </div>
        </Show>
        <div class="agentAuthorityActions"><button type="button" class="danger" onClick={killSwitch} disabled={busy() || Boolean(status()?.killSwitchActive)}>Revoke all / kill switch</button></div>
      </section>

      <section class="agentAuthoritySection" aria-labelledby="agent-grants-heading">
        <h3 id="agent-grants-heading">Exact grants</h3>
        <p class="ioDisclosureDescription">Selected: <code>{selectedPrincipal() || "none"}</code> / incarnation {selectedIncarnation() || "—"}</p>
        <div class="agentAuthorityForm agentAuthorityGrantForm">
          <label>Operation ID<input value={grantOperation()} onInput={(event) => setGrantOperation(event.currentTarget.value)} placeholder="syndocal.query...v1" /></label>
          <label>Capability<select value={grantCapability()} onChange={(event) => setGrantCapability(event.currentTarget.value as Capability)}><For each={capabilities}>{(value) => <option value={value}>{value}</option>}</For></select></label>
          <label>Project scope (optional)<input value={grantProject()} onInput={(event) => setGrantProject(event.currentTarget.value)} placeholder="unscoped" /></label>
          <button type="button" onClick={grant} disabled={busy()}>Install exact grant</button>
        </div>
      </section>

      <section class="agentAuthoritySection" aria-labelledby="agent-consent-heading">
        <h3 id="agent-consent-heading">Single-use operation consent</h3>
        <p class="ioDisclosureDescription">Prepare and consume consent only after the local operator has reviewed the exact operation. R4/R5 still require backend human-presence policy.</p>
        <div class="agentAuthorityForm agentAuthorityConsentForm">
          <label>Consent ID<input value={consentId()} onInput={(event) => setConsentId(event.currentTarget.value)} maxLength={128} /></label>
          <label>Principal<input value={consentPrincipal()} onInput={(event) => setConsentPrincipal(event.currentTarget.value)} maxLength={128} /></label>
          <label>Incarnation<input type="number" min="1" value={consentIncarnation()} onInput={(event) => setConsentIncarnation(event.currentTarget.value)} /></label>
          <label>Operation ID<input value={consentOperation()} onInput={(event) => setConsentOperation(event.currentTarget.value)} /></label>
          <label>Capability<select value={consentCapability()} onChange={(event) => setConsentCapability(event.currentTarget.value as Capability)}><For each={capabilities}>{(value) => <option value={value}>{value}</option>}</For></select></label>
          <label>Risk<select value={consentRisk()} onChange={(event) => setConsentRisk(event.currentTarget.value as Risk)}><For each={risks}>{(value) => <option value={value}>{value}</option>}</For></select></label>
          <label>Argument fingerprint (64 hex)<input value={consentDigest()} onInput={(event) => setConsentDigest(event.currentTarget.value)} maxLength={64} /></label>
          <label>Owner incarnation<input type="number" min="1" value={consentOwner()} onInput={(event) => setConsentOwner(event.currentTarget.value)} /></label>
          <label>Project generation<input type="number" min="0" value={consentProjectGeneration()} onInput={(event) => setConsentProjectGeneration(event.currentTarget.value)} /></label>
          <label>Output generation<input type="number" min="0" value={consentOutputGeneration()} onInput={(event) => setConsentOutputGeneration(event.currentTarget.value)} /></label>
          <div class="agentAuthorityActions">
            <button type="button" onClick={prepareConsent} disabled={busy()}>Prepare 15s consent</button>
            <button type="button" onClick={authorizeConsent} disabled={busy()}>Consume consent</button>
          </div>
        </div>
      </section>

      <section class="agentAuthoritySection" aria-labelledby="agent-audit-heading">
        <div class="agentAuthoritySectionHeading"><h3 id="agent-audit-heading">Audit viewer</h3><span>{status()?.audit.length ?? 0} / 512 retained</span></div>
        <Show when={status()?.audit.length} fallback={<p class="emptyState">No authority events recorded.</p>}>
          <div class="agentAuthorityAudit" role="log" aria-label="AI authority audit log">
            <For each={[...(status()?.audit ?? [])].reverse()}>{(record) => <div class="agentAuthorityAuditRow" data-agent-audit-sequence={record.sequence}>
              <span class="tabularNums">#{record.sequence}</span>
              <strong>{record.event}</strong>
              <span>{record.principalId ?? "all principals"}{record.principalIncarnation ? ` · inc ${record.principalIncarnation}` : ""}</span>
              <code>{record.operationId ?? "—"}</code>
              <span>{record.outcome}</span>
            </div>}</For>
          </div>
        </Show>
      </section>
    </aside>
  );
}
