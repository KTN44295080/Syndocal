import assert from "node:assert/strict";

const forcedStartControlCommands = [
  "live_audio_input_backends",
  "list_audio_input_devices",
  "get_live_audio_input_capabilities",
  "start_live_audio_input",
];

const forcedStartControlCallCounts = (calls, countLiveAudioRestoreCalls) =>
  Object.fromEntries(
    forcedStartControlCommands.map((command) => [
      command,
      countLiveAudioRestoreCalls(calls, command),
    ]),
  );

const assertForcedStartLeavesControlCallsUnchanged = (
  before,
  after,
  countLiveAudioRestoreCalls,
  description,
) => {
  const beforeCounts = forcedStartControlCallCounts(before.calls, countLiveAudioRestoreCalls);
  const afterCounts = forcedStartControlCallCounts(after.calls, countLiveAudioRestoreCalls);
  assert.deepEqual(
    afterCounts,
    beforeCounts,
    `${description} must not probe a backend/device/capability or dispatch Start`,
  );
  return true;
};

const persistedSavedSelectionFor = (backend) => JSON.stringify({
  schema_version: 1,
  backend,
  device_identity: backend === "asio"
    ? {
        backend: "ASIO",
        name: "Viewport ASIO Studio Driver",
        label: "Viewport ASIO Studio Driver · ASIO",
      }
    : {
        backend: "WASAPI",
        name: "Viewport Studio Microphone",
        label: "Viewport Studio Microphone · WASAPI",
      },
  sample_rate: 48_000,
  sample_format: "f32",
  stream_channels: 2,
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
});

async function runSavedCatalogueBarrierScenario(dependencies, {
  id,
  backend,
  backendCatalogueMode,
  expectedBarrierMessage,
  locale = "en",
  requiresExplicitAsioReselection = false,
}) {
  const {
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  } = dependencies;
  const rawSelection = persistedSavedSelectionFor(backend);
  const startup = await prepareLiveAudioRestoreViewport(client, viewport, rawSelection, locale);
  assert.equal(startup.storedBeforeMock, rawSelection, `${id} must begin with exact saved bytes`);
  await installLiveAudioInvokeMock(client);
  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("${id} live-audio mock was not installed.");
    mock.backendCatalogueMode = ${JSON.stringify(backendCatalogueMode)};
  })()`);
  const refreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    `${id} saved catalogue Refresh`,
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const backend = rail?.querySelector('[data-live-audio-control="backend"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return rail?.getAttribute('data-live-audio-backend') === ${JSON.stringify(backend)} &&
        backend?.value === ${JSON.stringify(backend)} &&
        saved?.getAttribute('data-live-audio-saved-state') === 'stale' &&
        saved?.textContent?.includes(${JSON.stringify(`SAVED ${backend === "asio" ? "ASIO" : "WASAPI"} LOCKED`)}) &&
        action instanceof HTMLButtonElement && action.disabled &&
        status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === 1 &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === 0 &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === 0;
    })()`,
    `${id} saved backend identity and catalogue barrier`,
  );
  const settled = await readLiveAudioRailState(client);
  const beforeForcedStart = await readLiveAudioRestoreMockAudit(client);
  const forcedStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    `${id} saved forced Start`,
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "live_audio_input_backends")
        } &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "list_audio_input_devices")
        } &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "get_live_audio_input_capabilities")
        } &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "start_live_audio_input")
        };
    })()`,
    `${id} saved direct Start preserves the catalogue barrier`,
  );
  const afterForcedStart = await readLiveAudioRestoreMockAudit(client);
  const storedAfterForcedStart = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  const forcedStartControlCallsUnchanged = assertForcedStartLeavesControlCallsUnchanged(
    beforeForcedStart,
    afterForcedStart,
    countLiveAudioRestoreCalls,
    `${id} saved forced Start`,
  );
  let recovery = null;
  if (requiresExplicitAsioReselection) {
    await client.evaluate(`(() => {
      const mock = window.__syndocalLiveAudioMock;
      if (!mock) throw new Error("${id} live-audio mock was not installed.");
      mock.backendCatalogueMode = "ready";
      mock.asioGeneration = 1;
    })()`);
    const recoveryRefreshTarget = await dispatchLiveAudioCdpControlClick(
      client,
      "refresh",
      `${id} recovered ASIO catalogue Refresh`,
    );
    await waitForClientCondition(
      client,
      `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
        const device = rail?.querySelector('[data-live-audio-control="device"]');
        const action = rail?.querySelector('[data-live-audio-action="transport"]');
        const saved = rail?.querySelector('[data-live-audio-saved-state]');
        return rail?.getAttribute('data-live-audio-backend') === 'asio' &&
          saved?.getAttribute('data-live-audio-saved-state') === 'stale' &&
          [...(device?.options ?? [])].some((option) => option.value === 'viewport-asio-studio-g2') &&
          action instanceof HTMLButtonElement && action.disabled;
      })()`,
      `${id} recovered ASIO remains locked pending explicit selection`,
    );
    const recovered = await readLiveAudioRailState(client);
    const deviceSelected = await setLiveAudioSelectWhenEnabled(
      client,
      "device",
      "viewport-asio-studio-g2",
      `${id} recovered exact ASIO driver`,
    );
    const rateSelected = await setLiveAudioSelectWhenEnabled(
      client,
      "rate",
      "48000",
      `${id} recovered ASIO rate`,
    );
    const bufferSelected = await setLiveAudioSelectWhenEnabled(
      client,
      "buffer",
      "128",
      `${id} recovered ASIO buffer`,
    );
    await waitForClientCondition(
      client,
      `document.querySelector('[data-live-audio-action="transport"]')?.disabled === false`,
      `${id} exact ASIO operator reselection unlocks Start`,
    );
    const reselected = await readLiveAudioRailState(client);
    recovery = {
      recoveryRefreshTarget,
      recovered,
      deviceSelected,
      rateSelected,
      bufferSelected,
      reselected,
    };
  }
  return {
    id,
    rawSelection,
    startup,
    refreshTarget,
    settled,
    beforeForcedStart,
    forcedStartTarget,
    afterForcedStart,
    storedAfterForcedStart,
    forcedStartControlCallsUnchanged,
    recovery,
    passed:
      startup.storedBeforeMock === rawSelection &&
      refreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      settled?.backend === backend &&
      settled?.savedSelectionState === "stale" &&
      settled?.actionDisabled === true &&
      settled?.appStatus === expectedBarrierMessage &&
      forcedStartTarget.forcedDirectHandlerProof === true &&
      forcedStartTarget.wasDisabled === true &&
      storedAfterForcedStart === rawSelection &&
      forcedStartControlCallsUnchanged &&
      (!requiresExplicitAsioReselection || (
        recovery?.recovered?.actionDisabled === true &&
        recovery.deviceSelected &&
        recovery.rateSelected &&
        recovery.bufferSelected &&
        recovery.reselected?.selectedDevice === "viewport-asio-studio-g2" &&
        recovery.reselected?.actionDisabled === false
      )),
  };
}

async function runSavedDeviceCatalogueFailureScenario(dependencies, {
  id,
  backend,
  deviceCatalogueMode,
  expectedBarrierMessage,
  expectedStaleDeviceLabel,
  recoveredDeviceId,
  locale = "en",
}) {
  const {
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  } = dependencies;
  const rawSelection = persistedSavedSelectionFor(backend);
  const initialDeviceId = backend === "asio"
    ? "viewport-asio-studio-g1"
    : "viewport-wasapi-studio-g1";
  const startup = await prepareLiveAudioRestoreViewport(client, viewport, rawSelection, locale);
  assert.equal(startup.storedBeforeMock, rawSelection, `${id} must begin with exact saved bytes`);
  await installLiveAudioInvokeMock(client);
  const readyRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    `${id} initial saved revalidation Refresh`,
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      return rail?.getAttribute('data-live-audio-backend') === ${JSON.stringify(backend)} &&
        device?.value === ${JSON.stringify(initialDeviceId)} &&
        saved?.getAttribute('data-live-audio-saved-state') === 'ready' &&
        action instanceof HTMLButtonElement && !action.disabled;
    })()`,
    `${id} initial saved selection is ready`,
  );
  const ready = await readLiveAudioRailState(client);
  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("${id} live-audio mock was not installed.");
    mock.deviceCatalogueMode = ${JSON.stringify(deviceCatalogueMode)};
  })()`);
  const failureRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    `${id} failed device catalogue Refresh`,
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      const stale = device?.querySelector('[data-live-audio-stale-device="true"]');
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return rail?.getAttribute('data-live-audio-backend') === ${JSON.stringify(backend)} &&
        device?.value === ${JSON.stringify(initialDeviceId)} &&
        stale?.textContent === ${JSON.stringify(expectedStaleDeviceLabel)} &&
        saved?.getAttribute('data-live-audio-saved-state') === 'stale' &&
        action instanceof HTMLButtonElement && action.disabled &&
        status === ${JSON.stringify(expectedBarrierMessage)};
    })()`,
    `${id} preserves visible exact identity after the device catalogue failure`,
  );
  const failed = await readLiveAudioRailState(client);
  const beforeForcedStart = await readLiveAudioRestoreMockAudit(client);
  const forcedStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    `${id} forced Start after failed device catalogue`,
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "live_audio_input_backends")
        } &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "list_audio_input_devices")
        } &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "get_live_audio_input_capabilities")
        } &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "start_live_audio_input")
        };
    })()`,
    `${id} forced Start keeps every live-audio IPC count unchanged`,
  );
  const afterForcedStart = await readLiveAudioRestoreMockAudit(client);
  const storedAfterForcedStart = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  const forcedStartControlCallsUnchanged = assertForcedStartLeavesControlCallsUnchanged(
    beforeForcedStart,
    afterForcedStart,
    countLiveAudioRestoreCalls,
    `${id} forced Start`,
  );
  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("${id} live-audio mock was not installed.");
    mock.deviceCatalogueMode = "ready";
  })()`);
  const recoveryRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    `${id} same-identity new-id recovery Refresh`,
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      return saved?.getAttribute('data-live-audio-saved-state') === 'stale' &&
        [...(device?.options ?? [])].some((option) => option.value === ${JSON.stringify(recoveredDeviceId)}) &&
        action instanceof HTMLButtonElement && action.disabled;
    })()`,
    `${id} same-identity new-id catalogue remains locked until explicit exact reselection`,
  );
  const recovered = await readLiveAudioRailState(client);
  const beforePassiveForcedStart = await readLiveAudioRestoreMockAudit(client);
  const passiveForcedStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    `${id} forced Start after same-identity passive recovery`,
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforePassiveForcedStart.calls, "live_audio_input_backends")
        } &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === ${
          countLiveAudioRestoreCalls(beforePassiveForcedStart.calls, "list_audio_input_devices")
        } &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
          countLiveAudioRestoreCalls(beforePassiveForcedStart.calls, "get_live_audio_input_capabilities")
        } &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          countLiveAudioRestoreCalls(beforePassiveForcedStart.calls, "start_live_audio_input")
        };
    })()`,
    `${id} same-identity passive recovery still keeps every live-audio IPC count unchanged`,
  );
  const afterPassiveForcedStart = await readLiveAudioRestoreMockAudit(client);
  const storedAfterPassiveForcedStart = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  const passiveForcedStartControlCallsUnchanged = assertForcedStartLeavesControlCallsUnchanged(
    beforePassiveForcedStart,
    afterPassiveForcedStart,
    countLiveAudioRestoreCalls,
    `${id} forced Start after same-identity passive recovery`,
  );
  const deviceSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "device",
    recoveredDeviceId,
    `${id} exact recovered device`,
  );
  const rateSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "rate",
    "48000",
    `${id} recovered sample rate`,
  );
  const bufferSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "buffer",
    "128",
    `${id} recovered buffer`,
  );
  try {
    await waitForClientCondition(
      client,
      `document.querySelector('[data-live-audio-action="transport"]')?.disabled === false`,
      `${id} exact reselection unlocks Start`,
    );
  } catch (error) {
    const evidence = await readLiveAudioRailState(client);
    const audit = await readLiveAudioRestoreMockAudit(client);
    throw new Error(`${String(error)}; state=${JSON.stringify(evidence)}; calls=${JSON.stringify(audit.calls)}`);
  }
  const reselected = await readLiveAudioRailState(client);
  return {
    id,
    rawSelection,
    startup,
    readyRefreshTarget,
    ready,
    failureRefreshTarget,
    failed,
    beforeForcedStart,
    forcedStartTarget,
    afterForcedStart,
    storedAfterForcedStart,
    forcedStartControlCallsUnchanged,
    recoveryRefreshTarget,
    recovered,
    beforePassiveForcedStart,
    passiveForcedStartTarget,
    afterPassiveForcedStart,
    storedAfterPassiveForcedStart,
    passiveForcedStartControlCallsUnchanged,
    deviceSelected,
    rateSelected,
    bufferSelected,
    reselected,
    passed:
      startup.storedBeforeMock === rawSelection &&
      readyRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      ready?.savedSelectionState === "ready" &&
      ready?.selectedDevice === initialDeviceId &&
      failureRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      failed?.savedSelectionState === "stale" &&
      failed?.selectedDevice === initialDeviceId &&
      failed?.deviceOptionLabels?.includes(expectedStaleDeviceLabel) &&
      failed?.actionDisabled === true &&
      failed?.appStatus === expectedBarrierMessage &&
      forcedStartTarget.forcedDirectHandlerProof === true &&
      forcedStartTarget.wasDisabled === true &&
      storedAfterForcedStart === rawSelection &&
      forcedStartControlCallsUnchanged &&
      recoveryRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      recovered?.savedSelectionState === "stale" &&
      recovered?.selectedDevice === initialDeviceId &&
      recovered?.deviceOptionLabels?.includes(expectedStaleDeviceLabel) &&
      recovered?.actionDisabled === true &&
      passiveForcedStartTarget.forcedDirectHandlerProof === true &&
      passiveForcedStartTarget.wasDisabled === true &&
      storedAfterPassiveForcedStart === rawSelection &&
      passiveForcedStartControlCallsUnchanged &&
      deviceSelected &&
      rateSelected &&
      bufferSelected &&
      reselected?.selectedDevice === recoveredDeviceId &&
      reselected?.actionDisabled === false,
  };
}

async function runColdSavedDeviceCatalogueFailureScenario(dependencies, {
  id,
  backend,
  deviceCatalogueMode,
  expectedBarrierMessage,
  expectedStaleDeviceId,
  expectedStaleDeviceLabel,
  locale = "en",
}) {
  const {
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    countLiveAudioRestoreCalls,
  } = dependencies;
  const rawSelection = persistedSavedSelectionFor(backend);
  const startup = await prepareLiveAudioRestoreViewport(client, viewport, rawSelection, locale);
  assert.equal(startup.storedBeforeMock, rawSelection, `${id} must begin with exact saved bytes`);
  await installLiveAudioInvokeMock(client);
  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("${id} live-audio mock was not installed.");
    mock.deviceCatalogueMode = ${JSON.stringify(deviceCatalogueMode)};
  })()`);
  const failureRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    `${id} first device catalogue Refresh`,
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      const stale = device?.querySelector('[data-live-audio-stale-device="true"]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return rail?.getAttribute('data-live-audio-backend') === ${JSON.stringify(backend)} &&
        device?.value === ${JSON.stringify(expectedStaleDeviceId)} &&
        stale?.textContent === ${JSON.stringify(expectedStaleDeviceLabel)} &&
        saved?.getAttribute('data-live-audio-saved-state') === 'stale' &&
        action instanceof HTMLButtonElement && action.disabled &&
        status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === 1 &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === 1 &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === 0 &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === 0;
    })()`,
    `${id} cold saved identity remains exactly visible and locked after the first failure`,
  );
  const failed = await readLiveAudioRailState(client);
  const beforeForcedStart = await readLiveAudioRestoreMockAudit(client);
  const forcedStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    `${id} forced Start after first device catalogue failure`,
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === ${JSON.stringify(expectedBarrierMessage)} &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "live_audio_input_backends")
        } &&
        calls.filter((call) => call.command === 'list_audio_input_devices').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "list_audio_input_devices")
        } &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "get_live_audio_input_capabilities")
        } &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          countLiveAudioRestoreCalls(beforeForcedStart.calls, "start_live_audio_input")
        };
    })()`,
    `${id} forced Start keeps every live-audio IPC count unchanged`,
  );
  const afterForcedStart = await readLiveAudioRestoreMockAudit(client);
  const storedAfterForcedStart = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  const forcedStartControlCallsUnchanged = assertForcedStartLeavesControlCallsUnchanged(
    beforeForcedStart,
    afterForcedStart,
    countLiveAudioRestoreCalls,
    `${id} forced Start`,
  );
  return {
    id,
    rawSelection,
    startup,
    failureRefreshTarget,
    failed,
    beforeForcedStart,
    forcedStartTarget,
    afterForcedStart,
    storedAfterForcedStart,
    forcedStartControlCallsUnchanged,
    passed:
      startup.storedBeforeMock === rawSelection &&
      failureRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      failed?.backend === backend &&
      failed?.selectedDevice === expectedStaleDeviceId &&
      failed?.deviceOptionLabels?.includes(expectedStaleDeviceLabel) &&
      failed?.savedSelectionState === "stale" &&
      failed?.actionDisabled === true &&
      failed?.appStatus === expectedBarrierMessage &&
      forcedStartTarget.forcedDirectHandlerProof === true &&
      forcedStartTarget.wasDisabled === true &&
      storedAfterForcedStart === rawSelection &&
      forcedStartControlCallsUnchanged,
  };
}

/**
 * FC-09 contract: backend refresh may never substitute another backend when the
 * current selection disappears. The runner injects its browser fixture helpers
 * so this contract remains focused and independently syntax-checkable.
 */
export async function runLiveAudioBackendDisappearanceContract({
  client,
  viewport,
  liveAudioInputSelectionStorageKey,
  prepareLiveAudioRestoreViewport,
  installLiveAudioInvokeMock,
  dispatchLiveAudioCdpControlClick,
  waitForClientCondition,
  readLiveAudioRailState,
  readLiveAudioRestoreMockAudit,
  setLiveAudioSelectWhenEnabled,
  countLiveAudioRestoreCalls,
}) {
  const startup = await prepareLiveAudioRestoreViewport(client, viewport, null);
  assert.equal(startup.storedBeforeMock, null, "FC-09 starts with no saved audio-input pin");
  await installLiveAudioInvokeMock(client);
  const initialRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    "FC-09 initial WASAPI discovery",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      return rail?.getAttribute('data-live-audio-backend') === 'wasapi_shared' &&
        [...(rail?.querySelector('[data-live-audio-control="device"]')?.options ?? [])]
          .some((option) => option.value === 'viewport-wasapi-studio-g1');
    })()`,
    "FC-09 initial ready WASAPI catalogue",
  );
  const wasapiDeviceSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "device",
    "viewport-wasapi-studio-g1",
    "FC-09 manual WASAPI device",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return calls.some((call) => call.command === 'get_live_audio_input_capabilities' &&
        call.args?.request?.backend === 'wasapiShared' &&
        call.args?.request?.deviceId === 'viewport-wasapi-studio-g1');
    })()`,
    "FC-09 manual WASAPI device capability resolution",
  );
  const manualWasapi = await readLiveAudioRailState(client);
  const storedBeforeWasapiDisappearance = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  assert.equal(
    storedBeforeWasapiDisappearance,
    null,
    "FC-09 manual WASAPI device alone must not create a saved backend pin",
  );
  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("FC-09 live-audio mock was not installed.");
    mock.backendCatalogueMode = "missing_wasapi";
  })()`);
  const beforeMissingWasapi = await readLiveAudioRestoreMockAudit(client);
  const missingWasapiRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    "FC-09 missing manual WASAPI backend Refresh",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const backend = rail?.querySelector('[data-live-audio-control="backend"]');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const staleDevice = device?.querySelector('[data-live-audio-stale-device="true"]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return rail?.getAttribute('data-live-audio-backend') === 'wasapi_shared' &&
        backend?.value === 'wasapi_shared' &&
        [...(backend?.options ?? [])].some((option) =>
          option.value === 'wasapi_shared' && option.textContent === 'WASAPI · selection unavailable') &&
        device?.value === 'viewport-wasapi-studio-g1' &&
        staleDevice?.textContent === 'Unavailable: Viewport Studio Microphone · WASAPI (viewport-wasapi-studio-g1)' &&
        action instanceof HTMLButtonElement && action.disabled &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeMissingWasapi.calls, "live_audio_input_backends") + 1
        };
    })()`,
    "FC-09 missing WASAPI backend retains manual identity without ASIO substitution",
  );
  const missingWasapi = await readLiveAudioRailState(client);
  const afterMissingWasapi = await readLiveAudioRestoreMockAudit(client);
  const startCountBeforeForcedMissingWasapiStart = countLiveAudioRestoreCalls(
    afterMissingWasapi.calls,
    "start_live_audio_input",
  );
  const forcedMissingWasapiStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    "FC-09 missing manual WASAPI forced Start",
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === 'Audio input backend wasapi_shared is absent from the current catalogue; devices and Start are locked.' &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          startCountBeforeForcedMissingWasapiStart
        };
    })()`,
    "FC-09 missing WASAPI direct Start terminal rejection",
  );
  const afterForcedMissingWasapiStart = await readLiveAudioRestoreMockAudit(client);
  const forcedMissingWasapiStartControlCallsUnchanged =
    assertForcedStartLeavesControlCallsUnchanged(
      afterMissingWasapi,
      afterForcedMissingWasapiStart,
      countLiveAudioRestoreCalls,
      "FC-09 missing manual WASAPI forced Start",
    );
  const asioDeviceListsAtMissingWasapi = countLiveAudioRestoreCalls(
    afterMissingWasapi.calls,
    "list_audio_input_devices",
    (call) => call.args?.request?.backend === "asio",
  );
  const asioBackendSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "backend",
    "asio",
    "FC-09 manual ASIO backend",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      return window.__syndocalLiveAudioMock?.asioGeneration === 1 &&
        rail?.getAttribute('data-live-audio-backend') === 'asio' &&
        [...(rail?.querySelector('[data-live-audio-control="device"]')?.options ?? [])]
          .some((option) => option.value === 'viewport-asio-studio-g1');
    })()`,
    "FC-09 manual ASIO device catalogue",
  );
  const asioDeviceSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "device",
    "viewport-asio-studio-g1",
    "FC-09 manual ASIO driver",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return calls.some((call) => call.command === 'get_live_audio_input_capabilities' &&
        call.args?.request?.backend === 'asio' &&
        call.args?.request?.deviceId === 'viewport-asio-studio-g1');
    })()`,
    "FC-09 manual ASIO driver capability resolution",
  );
  const manualAsio = await readLiveAudioRailState(client);
  const storedBeforeDisappearance = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  assert.equal(
    storedBeforeDisappearance,
    null,
    "FC-09 manual ASIO driver alone must not create a saved backend pin",
  );

  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("FC-09 live-audio mock was not installed.");
    mock.backendCatalogueMode = "missing_availability";
  })()`);
  const beforeMalformedCatalogue = await readLiveAudioRestoreMockAudit(client);
  const malformedCatalogueRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    "FC-09 malformed ASIO catalogue Refresh",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const backend = rail?.querySelector('[data-live-audio-control="backend"]');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return rail?.getAttribute('data-live-audio-backend') === 'asio' &&
        backend?.value === 'asio' &&
        device?.value === 'viewport-asio-studio-g1' &&
        action instanceof HTMLButtonElement && action.disabled &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeMalformedCatalogue.calls, "live_audio_input_backends") + 1
        };
    })()`,
    "FC-09 malformed ASIO catalogue retains typed rejection and stale identity",
  );
  await waitForClientCondition(
    client,
    `document.querySelector('.appStatusLine .appStatusText')?.textContent ===
      'Audio input backend asio has an invalid catalogue entry (AVAILABILITY_MISSING); devices and Start are locked.'`,
    "FC-09 malformed ASIO catalogue publishes its typed rejection",
  );
  const malformedCatalogue = await readLiveAudioRailState(client);
  const afterMalformedCatalogue = await readLiveAudioRestoreMockAudit(client);
  const startCountBeforeForcedMalformedCatalogueStart = countLiveAudioRestoreCalls(
    afterMalformedCatalogue.calls,
    "start_live_audio_input",
  );
  const forcedMalformedCatalogueStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    "FC-09 malformed ASIO catalogue forced Start",
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return calls.filter((call) => call.command === 'start_live_audio_input').length ===
        ${startCountBeforeForcedMalformedCatalogueStart};
    })()`,
    "FC-09 malformed ASIO catalogue direct Start preserves typed rejection",
  );
  const afterForcedMalformedCatalogueStart = await readLiveAudioRestoreMockAudit(client);
  const forcedMalformedCatalogueStartControlCallsUnchanged =
    assertForcedStartLeavesControlCallsUnchanged(
      afterMalformedCatalogue,
      afterForcedMalformedCatalogueStart,
      countLiveAudioRestoreCalls,
      "FC-09 malformed ASIO catalogue forced Start",
    );
  await waitForClientCondition(
    client,
    `document.querySelector('[data-live-audio-action="refresh"]')?.disabled === false`,
    "FC-09 malformed ASIO catalogue refresh settles before the next explicit probe",
  );

  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("FC-09 live-audio mock was not installed.");
    mock.backendCatalogueMode = "missing_asio";
  })()`);
  const beforeMissing = await readLiveAudioRestoreMockAudit(client);
  const missingRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    "FC-09 missing manual ASIO backend Refresh",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const backend = rail?.querySelector('[data-live-audio-control="backend"]');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const staleDevice = device?.querySelector('[data-live-audio-stale-device="true"]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return rail?.getAttribute('data-live-audio-backend') === 'asio' &&
        backend?.value === 'asio' &&
        [...(backend?.options ?? [])].some((option) =>
          option.value === 'asio' && option.textContent?.includes('selection unavailable')) &&
        device?.value === 'viewport-asio-studio-g1' &&
        staleDevice?.textContent === 'Unavailable: Viewport ASIO Studio Driver · ASIO (viewport-asio-studio-g1)' &&
        action instanceof HTMLButtonElement && action.disabled &&
        calls.filter((call) => call.command === 'live_audio_input_backends').length === ${
          countLiveAudioRestoreCalls(beforeMissing.calls, "live_audio_input_backends") + 1
        };
    })()`,
    "FC-09 missing ASIO backend retains manual exact identity",
  );
  const missing = await readLiveAudioRailState(client);
  const afterMissing = await readLiveAudioRestoreMockAudit(client);
  const storedAfterMissing = await client.evaluate(
    `window.localStorage.getItem(${JSON.stringify(liveAudioInputSelectionStorageKey)})`,
  );
  const startCountBeforeForcedMissingStart = countLiveAudioRestoreCalls(
    afterMissing.calls,
    "start_live_audio_input",
  );
  const forcedMissingStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    "FC-09 missing manual ASIO forced Start",
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      const status = document.querySelector('.appStatusLine .appStatusText')?.textContent ?? '';
      return status === 'Audio input backend asio is absent from the current catalogue; devices and Start are locked.' &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          startCountBeforeForcedMissingStart
        };
    })()`,
    "FC-09 missing ASIO direct Start terminal rejection",
  );
  const afterForcedMissingStart = await readLiveAudioRestoreMockAudit(client);
  const forcedMissingAsioStartControlCallsUnchanged =
    assertForcedStartLeavesControlCallsUnchanged(
      afterMissing,
      afterForcedMissingStart,
      countLiveAudioRestoreCalls,
      "FC-09 missing manual ASIO forced Start",
    );

  await client.evaluate(`(() => {
    const mock = window.__syndocalLiveAudioMock;
    if (!mock) throw new Error("FC-09 live-audio mock was not installed.");
    mock.backendCatalogueMode = "ready";
  })()`);
  const recoveredRefreshTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "refresh",
    "FC-09 recovered ASIO backend Refresh",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      return window.__syndocalLiveAudioMock?.asioGeneration === 2 &&
        rail?.getAttribute('data-live-audio-backend') === 'asio' &&
        device?.value === 'viewport-asio-studio-g1' &&
        device?.getAttribute('aria-invalid') === 'true' &&
        [...(device?.options ?? [])].some((option) => option.value === 'viewport-asio-studio-g2') &&
        action instanceof HTMLButtonElement && action.disabled;
    })()`,
    "FC-09 recovered ASIO requires exact manual driver reselection",
  );
  const recoveredStale = await readLiveAudioRailState(client);
  const recoveredDeviceSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "device",
    "viewport-asio-studio-g2",
    "FC-09 recovered exact ASIO driver",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return calls.some((call) => call.command === 'get_live_audio_input_capabilities' &&
        call.args?.request?.backend === 'asio' &&
        call.args?.request?.deviceId === 'viewport-asio-studio-g2');
    })()`,
    "FC-09 recovered exact ASIO driver capability resolution",
  );
  const recoveredRateSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "rate",
    "48000",
    "FC-09 recovered ASIO rate",
  );
  const recoveredBufferSelected = await setLiveAudioSelectWhenEnabled(
    client,
    "buffer",
    "128",
    "FC-09 recovered ASIO fixed buffer",
  );
  await waitForClientCondition(
    client,
    `document.querySelector('[data-live-audio-action="transport"]')?.disabled === false`,
    "FC-09 exact ASIO reselection unlocks Start",
  );
  const reselected = await readLiveAudioRailState(client);
  const beforeForgedStaleDeviceInput = await readLiveAudioRestoreMockAudit(client);
  const forgedStaleDeviceInput = await client.evaluate(`(() => {
    const device = document.querySelector('[data-live-audio-control="device"]');
    if (!(device instanceof HTMLSelectElement)) throw new Error('FC-09 device select is unavailable.');
    const forged = document.createElement('option');
    forged.value = 'viewport-asio-studio-g1';
    forged.textContent = 'forged stale ASIO device';
    device.append(forged);
    const priorValue = device.value;
    device.value = forged.value;
    device.dispatchEvent(new Event('input', { bubbles: true }));
    forged.remove();
    return { priorValue, dispatchedValue: 'viewport-asio-studio-g1' };
  })()`);
  await waitForClientCondition(
    client,
    `(() => {
      const rail = document.querySelector('.liveAudioInputBar');
      const device = rail?.querySelector('[data-live-audio-control="device"]');
      const action = rail?.querySelector('[data-live-audio-action="transport"]');
      const saved = rail?.querySelector('[data-live-audio-saved-state]');
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return device?.value === 'viewport-asio-studio-g1' &&
        device?.getAttribute('aria-invalid') === 'true' &&
        saved?.getAttribute('data-live-audio-saved-state') !== 'ready' &&
        action instanceof HTMLButtonElement && action.disabled &&
        calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
          countLiveAudioRestoreCalls(beforeForgedStaleDeviceInput.calls, "get_live_audio_input_capabilities")
        } &&
        calls.filter((call) => call.command === 'start_live_audio_input').length === ${
          countLiveAudioRestoreCalls(beforeForgedStaleDeviceInput.calls, "start_live_audio_input")
        };
    })()`,
    "FC-09 forged stale device input cannot probe or retain a ready Start state",
  );
  const forgedStaleDevice = await readLiveAudioRailState(client);
  const afterForgedStaleDeviceInput = await readLiveAudioRestoreMockAudit(client);
  const forgedStaleDeviceInputCapabilityCallsUnchanged =
    countLiveAudioRestoreCalls(
      afterForgedStaleDeviceInput.calls,
      "get_live_audio_input_capabilities",
    ) === countLiveAudioRestoreCalls(
      beforeForgedStaleDeviceInput.calls,
      "get_live_audio_input_capabilities",
    );
  const beforeForcedForgedStaleStart = await readLiveAudioRestoreMockAudit(client);
  const forcedForgedStaleStartTarget = await dispatchLiveAudioCdpControlClick(
    client,
    "transport",
    "FC-09 forged stale ASIO forced Start",
    { forceDisabledForDirectHandlerProof: true },
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalLiveAudioMock?.calls ?? [];
      return calls.filter((call) => call.command === 'get_live_audio_input_capabilities').length === ${
        countLiveAudioRestoreCalls(beforeForcedForgedStaleStart.calls, "get_live_audio_input_capabilities")
      } && calls.filter((call) => call.command === 'start_live_audio_input').length === ${
        countLiveAudioRestoreCalls(beforeForcedForgedStaleStart.calls, "start_live_audio_input")
      };
    })()`,
    "FC-09 forged stale ASIO forced Start cannot dispatch",
  );
  const afterForcedForgedStaleStart = await readLiveAudioRestoreMockAudit(client);
  const forgedStaleDeviceForcedStartControlCallsUnchanged =
    assertForcedStartLeavesControlCallsUnchanged(
      beforeForcedForgedStaleStart,
      afterForcedForgedStaleStart,
      countLiveAudioRestoreCalls,
      "FC-09 forged stale ASIO forced Start",
    );
  const finalAudit = afterForcedForgedStaleStart;
  const wasapiDeviceListsAtMissing = countLiveAudioRestoreCalls(
    afterMissing.calls,
    "list_audio_input_devices",
    (call) => call.args?.request?.backend === "wasapiShared",
  );
  const wasapiDeviceListsAfterReselection = countLiveAudioRestoreCalls(
    finalAudit.calls,
    "list_audio_input_devices",
    (call) => call.args?.request?.backend === "wasapiShared",
  );
  const savedAsioMissing = await runSavedCatalogueBarrierScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 saved ASIO missing backend",
    backend: "asio",
    backendCatalogueMode: "missing_asio",
    expectedBarrierMessage:
      "Audio input backend asio is absent from the current catalogue; devices and Start are locked.",
    requiresExplicitAsioReselection: true,
  });
  const savedAsioMalformed = await runSavedCatalogueBarrierScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 saved ASIO malformed catalogue",
    backend: "asio",
    backendCatalogueMode: "missing_availability",
    expectedBarrierMessage:
      "音声入力バックエンド asio のカタログ項目は無効です（AVAILABILITY_MISSING）。デバイスと開始はロックされています。",
    locale: "ja",
    requiresExplicitAsioReselection: true,
  });
  const savedWasapiMissing = await runSavedCatalogueBarrierScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 saved WASAPI missing backend",
    backend: "wasapi_shared",
    backendCatalogueMode: "missing_wasapi",
    expectedBarrierMessage:
      "Audio input backend wasapi_shared is absent from the current catalogue; devices and Start are locked.",
  });
  const coldSavedWasapiDeviceCatalogueReject = await runColdSavedDeviceCatalogueFailureScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 cold saved WASAPI device catalogue reject",
    backend: "wasapi_shared",
    deviceCatalogueMode: "reject",
    expectedBarrierMessage:
      "Audio input device catalogue for wasapi_shared failed (DEVICE_CATALOGUE_REQUEST_FAILED); devices and Start are locked.",
    expectedStaleDeviceId: "Viewport Studio Microphone",
    expectedStaleDeviceLabel:
      "Unavailable: Viewport Studio Microphone · WASAPI (Viewport Studio Microphone)",
  });
  const coldSavedAsioDeviceCatalogueMalformed = await runColdSavedDeviceCatalogueFailureScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 cold saved ASIO malformed device catalogue",
    backend: "asio",
    deviceCatalogueMode: "malformed",
    expectedBarrierMessage:
      "音声入力デバイスカタログ asio に失敗しました（DEVICE_CATALOGUE_ENTRY_INVALID）。デバイスと開始はロックされています。",
    expectedStaleDeviceId: "Viewport ASIO Studio Driver",
    expectedStaleDeviceLabel:
      "利用不可: Viewport ASIO Studio Driver · ASIO（Viewport ASIO Studio Driver）",
    locale: "ja",
  });
  const savedWasapiDeviceCatalogueReject = await runSavedDeviceCatalogueFailureScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 saved WASAPI device catalogue reject",
    backend: "wasapi_shared",
    deviceCatalogueMode: "reject",
    expectedBarrierMessage:
      "Audio input device catalogue for wasapi_shared failed (DEVICE_CATALOGUE_REQUEST_FAILED); devices and Start are locked.",
    expectedStaleDeviceLabel:
      "Unavailable: Viewport Studio Microphone · WASAPI (viewport-wasapi-studio-g1)",
    recoveredDeviceId: "viewport-wasapi-studio-g2",
  });
  const savedAsioDeviceCatalogueMalformed = await runSavedDeviceCatalogueFailureScenario({
    client,
    viewport,
    liveAudioInputSelectionStorageKey,
    prepareLiveAudioRestoreViewport,
    installLiveAudioInvokeMock,
    dispatchLiveAudioCdpControlClick,
    waitForClientCondition,
    readLiveAudioRailState,
    readLiveAudioRestoreMockAudit,
    setLiveAudioSelectWhenEnabled,
    countLiveAudioRestoreCalls,
  }, {
    id: "FC-09 saved ASIO malformed device catalogue",
    backend: "asio",
    deviceCatalogueMode: "malformed",
    expectedBarrierMessage:
      "音声入力デバイスカタログ asio に失敗しました（DEVICE_CATALOGUE_ENTRY_INVALID）。デバイスと開始はロックされています。",
    expectedStaleDeviceLabel:
      "利用不可: Viewport ASIO Studio Driver · ASIO（viewport-asio-studio-g1）",
    recoveredDeviceId: "viewport-asio-studio-g2",
    locale: "ja",
  });
  const checks = {
    startsWithAnUnsavedManualAsioSelection:
      startup.storedBeforeMock === null &&
      manualAsio?.backend === "asio" &&
      manualAsio?.selectedDevice === "viewport-asio-studio-g1" &&
      manualAsio?.selectedSampleRate === "" &&
      manualAsio?.selectedBuffer === "" &&
      manualAsio?.savedSelectionState === "" &&
      storedBeforeDisappearance === null,
    missingBackendRetainsVisibleExactAsioIdentity:
      missingRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      missing?.backend === "asio" &&
      missing?.backendOptions?.[0] === "asio" &&
      missing?.backendOptionLabels?.[0] === "ASIO · selection unavailable" &&
      missing?.selectedDevice === "viewport-asio-studio-g1" &&
      missing?.deviceOptionLabels?.includes(
        "Unavailable: Viewport ASIO Studio Driver · ASIO (viewport-asio-studio-g1)",
      ) &&
      missing?.actionDisabled === true &&
      missing?.savedSelectionState === "" &&
      storedAfterMissing === null,
    missingBackendCannotFallbackOrStart:
      countLiveAudioRestoreCalls(
        afterMissing.calls,
        "list_audio_input_devices",
        (call) => call.args?.request?.backend === "wasapiShared",
      ) === wasapiDeviceListsAtMissing &&
      forcedMissingStartTarget.forcedDirectHandlerProof === true &&
      forcedMissingStartTarget.wasDisabled === true &&
      missing?.appStatus === "Audio input backend asio is absent from the current catalogue; devices and Start are locked." &&
      forcedMissingAsioStartControlCallsUnchanged &&
      countLiveAudioRestoreCalls(afterForcedMissingStart.calls, "start_live_audio_input") ===
        startCountBeforeForcedMissingStart,
    recoveredBackendStillRequiresExactOperatorReselection:
      recoveredRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      recoveredStale?.backend === "asio" &&
      recoveredStale?.selectedDevice === "viewport-asio-studio-g1" &&
      recoveredStale?.deviceInvalid === "true" &&
      recoveredStale?.deviceOptions?.includes("viewport-asio-studio-g2") &&
      recoveredStale?.actionDisabled === true,
    exactRefreshedAsioReselectionIsTheOnlyUnlock:
      recoveredDeviceSelected &&
      recoveredRateSelected &&
      recoveredBufferSelected &&
      reselected?.backend === "asio" &&
      reselected?.selectedDevice === "viewport-asio-studio-g2" &&
      reselected?.selectedSampleRate === "48000" &&
      reselected?.selectedBuffer === "128" &&
      reselected?.deviceInvalid === "" &&
      reselected?.actionDisabled === false &&
      wasapiDeviceListsAfterReselection === wasapiDeviceListsAtMissing,
    forgedStaleDeviceInputCannotProbeOrRetainReadyStart:
      forgedStaleDeviceInput?.priorValue === "viewport-asio-studio-g2" &&
      forgedStaleDeviceInput?.dispatchedValue === "viewport-asio-studio-g1" &&
      forgedStaleDevice?.selectedDevice === "viewport-asio-studio-g1" &&
      forgedStaleDevice?.deviceInvalid === "true" &&
      forgedStaleDevice?.savedSelectionState !== "ready" &&
      forgedStaleDevice?.actionDisabled === true &&
      forgedStaleDeviceInputCapabilityCallsUnchanged &&
      forcedForgedStaleStartTarget.forcedDirectHandlerProof === true &&
      forcedForgedStaleStartTarget.wasDisabled === true &&
      forgedStaleDeviceForcedStartControlCallsUnchanged,
    missingWasapiRetainsIdentityWithoutAsioOrFirstSubstitution:
      wasapiDeviceSelected &&
      manualWasapi?.backend === "wasapi_shared" &&
      manualWasapi?.selectedDevice === "viewport-wasapi-studio-g1" &&
      storedBeforeWasapiDisappearance === null &&
      missingWasapiRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      missingWasapi?.backend === "wasapi_shared" &&
      missingWasapi?.backendOptions?.[0] === "wasapi_shared" &&
      missingWasapi?.backendOptionLabels?.[0] === "WASAPI · selection unavailable" &&
      missingWasapi?.selectedDevice === "viewport-wasapi-studio-g1" &&
      missingWasapi?.deviceOptionLabels?.includes(
        "Unavailable: Viewport Studio Microphone · WASAPI (viewport-wasapi-studio-g1)",
      ) &&
      missingWasapi?.actionDisabled === true &&
      forcedMissingWasapiStartTarget.forcedDirectHandlerProof === true &&
      forcedMissingWasapiStartTarget.wasDisabled === true &&
      missingWasapi?.appStatus === "Audio input backend wasapi_shared is absent from the current catalogue; devices and Start are locked." &&
      forcedMissingWasapiStartControlCallsUnchanged &&
      countLiveAudioRestoreCalls(afterForcedMissingWasapiStart.calls, "start_live_audio_input") ===
        startCountBeforeForcedMissingWasapiStart &&
      asioDeviceListsAtMissingWasapi === 0,
    malformedCatalogueKeepsTypedReasonAndBlocksStart:
      malformedCatalogueRefreshTarget.pointerSequence === "mouseMoved>mousePressed>mouseReleased" &&
      malformedCatalogue?.backend === "asio" &&
      malformedCatalogue?.selectedDevice === "viewport-asio-studio-g1" &&
      malformedCatalogue?.deviceOptionLabels?.includes(
        "Unavailable: Viewport ASIO Studio Driver · ASIO (viewport-asio-studio-g1)",
      ) &&
      malformedCatalogue?.actionDisabled === true &&
      malformedCatalogue?.appStatus === "Audio input backend asio has an invalid catalogue entry (AVAILABILITY_MISSING); devices and Start are locked." &&
      forcedMalformedCatalogueStartTarget.forcedDirectHandlerProof === true &&
      forcedMalformedCatalogueStartTarget.wasDisabled === true &&
      forcedMalformedCatalogueStartControlCallsUnchanged &&
      countLiveAudioRestoreCalls(afterForcedMalformedCatalogueStart.calls, "start_live_audio_input") ===
        startCountBeforeForcedMalformedCatalogueStart,
    savedSelectionsPreserveCatalogueBarriersAndRequireExplicitAsioReselection:
      savedAsioMissing.passed &&
      savedAsioMalformed.passed &&
      savedWasapiMissing.passed,
    savedReadySelectionsFailClosedOnDeviceCatalogueFailure:
      savedWasapiDeviceCatalogueReject.passed &&
      savedAsioDeviceCatalogueMalformed.passed,
    coldSavedSelectionsRetainExactStableIdentityOnFirstDeviceCatalogueFailure:
      coldSavedWasapiDeviceCatalogueReject.passed &&
      coldSavedAsioDeviceCatalogueMalformed.passed,
  };
  const failedChecks = Object.entries(checks)
    .filter(([, passed]) => !passed)
    .map(([name]) => name);
  return {
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    startup,
    initialRefreshTarget,
    manualAsio,
    missingRefreshTarget,
    missing,
    forcedMissingStartTarget,
    recoveredRefreshTarget,
    recoveredStale,
    reselected,
    forgedStaleDeviceInput,
    forgedStaleDevice,
    forcedForgedStaleStartTarget,
    savedAsioMissing,
    savedAsioMalformed,
    savedWasapiMissing,
    coldSavedWasapiDeviceCatalogueReject,
    coldSavedAsioDeviceCatalogueMalformed,
    savedWasapiDeviceCatalogueReject,
    savedAsioDeviceCatalogueMalformed,
    calls: finalAudit.calls,
  };
}
