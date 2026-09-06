/* Focused browser contract for the connection-first Setup I/O operator deck. */

const expectedZoneByConnection = {
  dmx: "dmx",
  audio: "audio",
  midi: "midi",
  osc: "osc",
  web: "remote",
  dj: "remote",
};

async function clickIoConnectionTab(client, connectionId) {
  const geometry = await client.evaluate(`(() => {
    const card = document.querySelector('[data-io-primary-connection="${connectionId}"]');
    const button = card?.querySelector(':scope > button[role="tab"][aria-controls]');
    if (!(button instanceof HTMLElement)) return { found: false, hitTestable: false };
    button.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    const rect = button.getBoundingClientRect();
    const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return {
      found: rect.width > 0 && rect.height > 0,
      hitTestable: target === button || button.contains(target),
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
    };
  })()`);
  if (!geometry.found || !geometry.hitTestable) {
    throw new Error(`Setup I/O ${connectionId} selector is not hit-testable: ${JSON.stringify(geometry)}`);
  }
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: geometry.x, y: geometry.y });
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: geometry.x,
    y: geometry.y,
    button: "left",
    buttons: 1,
    clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: geometry.x,
    y: geometry.y,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
  return geometry;
}

export async function selectSetupIoConnection(client, connectionId, waitForClientCondition) {
  await clickIoConnectionTab(client, connectionId);
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="${connectionId}"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.querySelector('[data-io-workbench-body]')?.querySelectorAll('[data-io-zone]').length === 1`,
    `Setup I/O ${connectionId} workbench selection`,
  );
}

async function exerciseIoWorkbenchDraftContinuity(client, evaluatePageFunction) {
  return await evaluatePageFunction(client, async () => {
    const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const pause = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
    const activeConnection = () => document
      .querySelector('[data-io-primary-connection] > button[role="tab"][aria-selected="true"]')
      ?.closest('[data-io-primary-connection]')
      ?.getAttribute('data-io-primary-connection') ?? null;
    const activeConnectionState = () => document
      .querySelector('[data-io-primary-connection] > button[role="tab"][aria-selected="true"]')
      ?.closest('[data-io-primary-connection]')
      ?.querySelector(':scope > .setupIoConnectionMeta .setupIoConnectionState')
      ?.textContent
      ?.trim() ?? null;
    const waitForConnectionState = async (expected) => {
      const deadline = performance.now() + 2_400;
      while (performance.now() < deadline) {
        if (activeConnectionState() === expected) return true;
        await pause(80);
      }
      return activeConnectionState() === expected;
    };
    const readDraft = () => document.querySelector('[data-io-control="dj-track-title-contains"]');
    const draftBefore = readDraft();
    const refresh = document.querySelector('[data-io-control="dj-link-refresh-wired-candidates"]');
    const initialConnectionState = activeConnectionState();
    const mock = window.__syndocalSetupIoMock;
    if (!(draftBefore instanceof HTMLInputElement) || !(refresh instanceof HTMLButtonElement) || refresh.disabled) {
      return { passed: false, reason: 'DJ Link draft or wired refresh control is unavailable' };
    }
    if (!mock || typeof mock.setDjLinkListenerRunning !== 'function' || !initialConnectionState) {
      return { passed: false, reason: 'Setup I/O status continuity mock is unavailable' };
    }

    draftBefore.value = 'workbench-continuity-probe';
    draftBefore.dispatchEvent(new Event('input', { bubbles: true }));
    const activeBefore = activeConnection();
    mock.djLinkWiredCandidates = [{
      adapterGuid: '44444444-4444-4444-4444-444444444444',
      networkGuid: '55555555-5555-5555-5555-555555555555',
      bindIp: '192.168.50.1',
      adapterAlias: 'Ethernet4',
    }];
    mock.djLinkWiredCandidatesDelayMs = 80;
    mock.djLinkWiredCandidatesError = null;
    refresh.click();
    await settle();
    const draftDuring = readDraft();
    const activeDuring = activeConnection();
    await new Promise((resolve) => setTimeout(resolve, 120));
    const draftAfter = readDraft();
    const activeAfter = activeConnection();

    mock.setDjLinkListenerRunning(true);
    const connectedStateReached = await waitForConnectionState('Connected');
    const draftDuringStatus = readDraft();
    const activeDuringStatus = activeConnection();
    const stateDuringStatus = activeConnectionState();
    mock.setDjLinkListenerRunning(false);
    const restoredStateReached = await waitForConnectionState(initialConnectionState);
    const draftAfterStatusRestore = readDraft();
    const activeAfterStatusRestore = activeConnection();
    const stateAfterStatusRestore = activeConnectionState();

    document.querySelector('[data-io-primary-connection="midi"] > button[role="tab"]')?.click();
    await settle();
    const activeMidi = activeConnection();
    const midiZone = document.querySelector('[data-io-workbench-body] [data-io-zone="midi"]');
    document.querySelector('[data-io-primary-connection="dj"] > button[role="tab"]')?.click();
    await settle();
    const activeDjAfterSwitch = activeConnection();
    const draftAfterConnectionSwitch = readDraft();

    mock.djLinkWiredCandidates = [];
    mock.djLinkWiredCandidatesDelayMs = 0;
    mock.djLinkWiredCandidatesError = null;
    const refreshAfterConnectionSwitch = document.querySelector('[data-io-control="dj-link-refresh-wired-candidates"]');
    if (refreshAfterConnectionSwitch instanceof HTMLButtonElement && !refreshAfterConnectionSwitch.disabled) {
      refreshAfterConnectionSwitch.click();
      await settle();
      await new Promise((resolve) => setTimeout(resolve, 40));
    }
    const restoredSelect = document.querySelector('[data-io-control="dj-link-wired-binding"]');
    const restoredCandidateCount = restoredSelect instanceof HTMLSelectElement
      ? [...restoredSelect.options].filter((option) => option.value !== '').length
      : -1;
    const restoredRefresh = document.querySelector('[data-io-control="dj-link-refresh-wired-candidates"]');
    const restoredError = document.querySelector('[data-io-status="dj-link-wired-refresh-error"]');
    return {
      sameNodeDuringStatusUpdate: draftBefore === draftDuring,
      sameNodeAfterStatusUpdate: draftBefore === draftAfter,
      sameNodeDuringListenerStatusUpdate: draftBefore === draftDuringStatus,
      sameNodeAfterListenerStatusRestore: draftBefore === draftAfterStatusRestore,
      valueAfterStatusUpdate: draftAfter instanceof HTMLInputElement ? draftAfter.value : '',
      valueAfterListenerStatusUpdate: draftDuringStatus instanceof HTMLInputElement ? draftDuringStatus.value : '',
      valueAfterListenerStatusRestore: draftAfterStatusRestore instanceof HTMLInputElement ? draftAfterStatusRestore.value : '',
      activeBefore,
      activeDuring,
      activeAfter,
      activeDuringStatus,
      activeAfterStatusRestore,
      connectionStateBefore: initialConnectionState,
      connectionStateDuringStatus: stateDuringStatus,
      connectionStateAfterStatusRestore: stateAfterStatusRestore,
      listenerStatusChanged: connectedStateReached && stateDuringStatus === 'Connected',
      listenerStatusRestored: restoredStateReached && stateAfterStatusRestore === initialConnectionState,
      activeMidi,
      activeDjAfterSwitch,
      midiZoneMounted: Boolean(midiZone),
      connectionSwitchReplacesWorkbench: draftAfterConnectionSwitch !== draftBefore,
      restoredCandidateCount,
      restoredRefreshIdle: restoredRefresh instanceof HTMLButtonElement && !restoredRefresh.disabled,
      restoredErrorAbsent: !restoredError,
      passed:
        initialConnectionState === 'Disarmed' &&
        activeBefore === 'dj' &&
        activeDuring === 'dj' &&
        activeAfter === 'dj' &&
        activeMidi === 'midi' &&
        activeDjAfterSwitch === 'dj' &&
        Boolean(midiZone) &&
        draftBefore === draftDuring &&
        draftBefore === draftAfter &&
        draftAfter instanceof HTMLInputElement &&
        draftAfter.value === 'workbench-continuity-probe' &&
        connectedStateReached &&
        stateDuringStatus === 'Connected' &&
        activeDuringStatus === 'dj' &&
        draftBefore === draftDuringStatus &&
        draftDuringStatus instanceof HTMLInputElement &&
        draftDuringStatus.value === 'workbench-continuity-probe' &&
        restoredStateReached &&
        stateAfterStatusRestore === initialConnectionState &&
        activeAfterStatusRestore === 'dj' &&
        draftBefore === draftAfterStatusRestore &&
        draftAfterStatusRestore instanceof HTMLInputElement &&
        draftAfterStatusRestore.value === 'workbench-continuity-probe' &&
        draftAfterConnectionSwitch instanceof HTMLInputElement &&
        draftAfterConnectionSwitch !== draftBefore &&
        restoredCandidateCount === 0 &&
        restoredRefresh instanceof HTMLButtonElement &&
        !restoredRefresh.disabled &&
        !restoredError,
    };
  });
}

export async function exerciseSetupIoOperatorDeck(client, helpers) {
  const { evaluatePageFunction, pressKey, sleep, waitForClientCondition } = helpers;
  const selectIoConnection = async (connectionId) => {
    await selectSetupIoConnection(client, connectionId, waitForClientCondition);
  };
  const readState = async (expectedId, probeOverflow = false) => evaluatePageFunction(
    client,
    async (wantedId, shouldProbeOverflow, zoneByConnection) => {
      const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const visible = (element) => {
        if (!(element instanceof HTMLElement)) return false;
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden";
      };
      const hitTestable = (element) => {
        if (!visible(element)) return false;
        const rect = element.getBoundingClientRect();
        const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
        return target === element || element.contains(target);
      };
      const deck = document.querySelector("[data-io-connection-deck]");
      const tablist = deck?.querySelector(':scope > [role="tablist"]');
      const panel = deck?.querySelector(':scope > [role="tabpanel"]');
      const body = panel?.querySelector(":scope > [data-io-workbench-body]");
      const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      const cards = [...(deck?.querySelectorAll("[data-io-primary-connection]") ?? [])].map((card) => ({
        id: card.getAttribute("data-io-primary-connection") ?? "",
        card,
        button: card.querySelector(':scope > button[role="tab"][aria-controls]'),
        meta: card.querySelector(":scope > .setupIoConnectionMeta"),
        action: card.querySelector(":scope > .setupIoConnectionMeta > .setupIoConnectionAction > button"),
      }));
      const selectedCards = cards.filter(({ button }) => button?.getAttribute("aria-selected") === "true");
      const tabs = cards.map(({ button }) => button);
      const bodyRect = body?.getBoundingClientRect() ?? null;
      const panelRect = panel?.getBoundingClientRect() ?? null;
      const zoneNodes = body ? [...body.querySelectorAll("[data-io-zone]")].filter(visible) : [];
      const zoneRect = zoneNodes[0]?.getBoundingClientRect() ?? null;
      const bodyStyle = body ? getComputedStyle(body) : null;
      const bodyHorizontalPadding = bodyStyle
        ? parseFloat(bodyStyle.paddingLeft || "0") + parseFloat(bodyStyle.paddingRight || "0")
        : 0;
      const bodyContentWidth = bodyRect ? bodyRect.width - bodyHorizontalPadding : 0;
      let bodyOverflowProbePassed = false;
      if (shouldProbeOverflow && body instanceof HTMLElement) {
        const probe = document.createElement("div");
        probe.dataset.ioOverflowProbe = "true";
        probe.style.cssText = "height: 1600px; width: 1px; flex: none; pointer-events: none;";
        body.append(probe);
        await settle();
        bodyOverflowProbePassed = body.scrollHeight > body.clientHeight;
        probe.remove();
        await settle();
      }
      const nestedScrollports = body instanceof HTMLElement
        ? [...body.querySelectorAll("*")].filter((element) => {
          if (!(element instanceof HTMLElement)) return false;
          const style = getComputedStyle(element);
          return (/(auto|scroll)/.test(style.overflowY) || /(auto|scroll)/.test(style.overflowX)) &&
            (element.scrollHeight > element.clientHeight || element.scrollWidth > element.clientWidth);
        })
        : [];
      const nestedOverflowStyles = body instanceof HTMLElement
        ? [...body.querySelectorAll("*")].filter((element) => {
          if (!(element instanceof HTMLElement)) return false;
          return /(auto|scroll)/.test(getComputedStyle(element).overflowY);
        })
        : [];
      const cardMetrics = cards.map(({ id, card, button, meta, action }) => ({
        id,
        visible: visible(card),
        hitTestable: hitTestable(button),
        focusable: (() => {
          if (!(button instanceof HTMLElement)) return false;
          button.focus();
          return document.activeElement === button;
        })(),
        selected: button?.getAttribute("aria-selected") === "true",
        actionFound: action instanceof HTMLElement,
        actionIsSibling: Boolean(
          button?.parentElement === card && meta?.parentElement === card && action && !button.contains(action),
        ),
        actionHitTestable: hitTestable(action),
        zoneNames: zoneNodes.map((zone) => zone.getAttribute("data-io-zone") ?? ""),
        expectedZone: zoneByConnection[id] ?? null,
        exactlyOneZone: zoneNodes.length === 1,
        workbenchFullWidth: Boolean(
          bodyRect && panelRect && zoneRect && bodyRect.width > 0 &&
          zoneRect.width >= bodyContentWidth - 2 && panelRect.width >= bodyRect.width - 2,
        ),
        bodyScrollable: Boolean(
          body instanceof HTMLElement && bodyStyle && /(auto|scroll)/.test(bodyStyle.overflowY) && bodyOverflowProbePassed,
        ),
      }));
      if (previouslyFocused) previouslyFocused.focus();
      return {
        cardCount: cards.length,
        tablistRole: tablist?.getAttribute("role") ?? null,
        tablistLabel: tablist?.getAttribute("aria-label") ?? null,
        tabRoles: tabs.map((tab) => tab?.getAttribute("role") ?? null),
        tabIds: tabs.map((tab) => tab?.getAttribute("id") ?? null),
        tabSelected: tabs.map((tab) => tab?.getAttribute("aria-selected") ?? null),
        tabIndex: tabs.map((tab) => tab?.getAttribute("tabindex") ?? null),
        tabControls: tabs.map((tab) => tab?.getAttribute("aria-controls") ?? null),
        panelRole: panel?.getAttribute("role") ?? null,
        panelId: panel?.getAttribute("id") ?? null,
        panelLabelledBy: panel?.getAttribute("aria-labelledby") ?? null,
        panelConnection: panel?.getAttribute("data-io-connection-workbench") ?? null,
        panelConnectionId: panel?.getAttribute("data-io-connection-workbench") ?? null,
        obsoleteHeaderCount: panel?.querySelectorAll(":scope > .setupIoWorkbenchHeader").length ?? 0,
        selectedIds: selectedCards.map(({ id }) => id),
        cards: cardMetrics,
        bodyOverflowProbePassed,
        nestedScrollportCount: nestedScrollports.length,
        nestedOverflowStyleCount: nestedOverflowStyles.length,
      };
    },
    expectedId,
    probeOverflow,
    expectedZoneByConnection,
  );

  const cardResults = [];
  let lastTabPointerHitTestable = false;
  for (const id of Object.keys(expectedZoneByConnection)) {
    const geometry = await clickIoConnectionTab(client, id);
    if (id === "dj") lastTabPointerHitTestable = geometry.hitTestable;
    await waitForClientCondition(
      client,
      `document.querySelector('[data-io-primary-connection="${id}"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true'`,
      `Setup I/O ${id} selector activation`,
    );
    const state = await readState(id, true);
    const result = state.cards.find((card) => card.id === id);
    cardResults.push({
      ...(result ?? {
        id,
        visible: false,
        hitTestable: false,
        focusable: false,
        selected: false,
        actionFound: false,
        actionIsSibling: false,
        actionHitTestable: false,
        exactlyOneZone: false,
        zoneNames: [],
        expectedZone: expectedZoneByConnection[id],
        workbenchFullWidth: false,
        bodyScrollable: false,
      }),
      bodyOverflowProbePassed: state.bodyOverflowProbePassed,
      nestedScrollportCount: state.nestedScrollportCount,
      nestedOverflowStyleCount: state.nestedOverflowStyleCount,
      tabpanelRelationExact: state.tablistRole === "tablist" &&
        state.panelRole === "tabpanel" && Boolean(state.panelId) &&
        state.tabRoles.every((role) => role === "tab") &&
        state.tabControls.every((controls) => controls === state.panelId) &&
        state.tabSelected.filter((selected) => selected === "true").length === 1 &&
        state.panelLabelledBy === state.tabIds.find((tabId, index) => state.tabSelected[index] === "true"),
    });
  }

  // DMX no longer exposes an inactive-card quick action. Verify the current
  // operator path instead: while another workbench is active, the DMX tab
  // remains reachable and selecting it mounts the DMX workbench/zone.
  await selectIoConnection("dj");
  const activeBeforeDmxTab = await evaluatePageFunction(client, () =>
    document.querySelector('[data-io-primary-connection] > button[role="tab"][aria-selected="true"]')
      ?.closest("[data-io-primary-connection]")?.getAttribute("data-io-primary-connection") ?? null,
  );
  const dmxTabGeometry = await client.evaluate(`(() => {
    const tab = document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]');
    if (!(tab instanceof HTMLElement)) return { found: false, hitTestable: false };
    tab.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    const rect = tab.getBoundingClientRect();
    const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return {
      found: rect.width > 0 && rect.height > 0,
      hitTestable: target === tab || tab.contains(target),
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
    };
  })()`);
  if (!dmxTabGeometry.found || !dmxTabGeometry.hitTestable) {
    throw new Error(`Setup I/O DMX tab is not hit-testable: ${JSON.stringify(dmxTabGeometry)}`);
  }
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: dmxTabGeometry.x, y: dmxTabGeometry.y });
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed", x: dmxTabGeometry.x, y: dmxTabGeometry.y,
    button: "left", buttons: 1, clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased", x: dmxTabGeometry.x, y: dmxTabGeometry.y,
    button: "left", buttons: 0, clickCount: 1,
  });
  await sleep(80);
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.querySelector('[data-io-connection-workbench="dmx"] [data-io-zone="dmx"]') !== null`,
    "Setup I/O DMX tab workbench selection",
  );
  const dmxTabState = await readState("dmx");
  const dmxTabCard = dmxTabState.cards.find((card) => card.id === "dmx");
  const dmxTabReachesWorkbench =
    activeBeforeDmxTab === "dj" &&
    dmxTabState.selectedIds.length === 1 &&
    dmxTabState.selectedIds[0] === "dmx" &&
    dmxTabState.panelConnection === "dmx" &&
    dmxTabCard?.zoneNames.length === 1 &&
    dmxTabCard.zoneNames[0] === "dmx";

  await selectIoConnection("dmx");
  const dmxPointerFocus = await evaluatePageFunction(client, () => {
    const button = document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]');
    return button instanceof HTMLElement && document.activeElement === button;
  });
  await pressKey(client, "ArrowRight", "ArrowRight");
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="audio"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.activeElement === document.querySelector('[data-io-primary-connection="audio"] > button[role="tab"]')`,
    "Setup I/O ArrowRight keyboard selection of Audio",
  );
  const arrowRightAudioState = await readState("audio");
  await pressKey(client, "ArrowRight", "ArrowRight");
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="midi"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.activeElement === document.querySelector('[data-io-primary-connection="midi"] > button[role="tab"]')`,
    "Setup I/O ArrowRight keyboard selection",
  );
  const arrowRightState = await readState("midi");
  await pressKey(client, "End", "End");
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="dj"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.activeElement === document.querySelector('[data-io-primary-connection="dj"] > button[role="tab"]')`,
    "Setup I/O End keyboard selection",
  );
  const endState = await readState("dj");
  await pressKey(client, "Home", "Home");
  await waitForClientCondition(
    client,
    `document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true' && document.activeElement === document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]')`,
    "Setup I/O Home keyboard selection",
  );
  const homeState = await readState("dmx");
  await evaluatePageFunction(client, () => document.querySelector('[data-io-primary-connection="midi"] > button[role="tab"]')?.focus());
  await pressKey(client, "Enter");
  await waitForClientCondition(client, `document.querySelector('[data-io-primary-connection="midi"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true'`, "Setup I/O Enter keyboard selection");
  const enterState = await readState("midi");
  await evaluatePageFunction(client, () => document.querySelector('[data-io-primary-connection="osc"] > button[role="tab"]')?.focus());
  await pressKey(client, "Space", " ");
  await waitForClientCondition(client, `document.querySelector('[data-io-primary-connection="osc"] > button[role="tab"]')?.getAttribute('aria-selected') === 'true'`, "Setup I/O Space keyboard selection");
  const spaceState = await readState("osc");
  await selectIoConnection("dj");
  const draftContinuity = await exerciseIoWorkbenchDraftContinuity(client, evaluatePageFunction);
  const finalState = await readState("dj", true);
  const exactTabpanelRelation = finalState.tablistRole === "tablist" &&
    finalState.panelRole === "tabpanel" && Boolean(finalState.panelId) &&
    finalState.tabIds.length === new Set(finalState.tabIds).size &&
    finalState.tabRoles.every((role) => role === "tab") &&
    finalState.tabControls.every((controls) => controls === finalState.panelId) &&
    finalState.tabSelected.filter((selected) => selected === "true").length === 1 &&
    finalState.panelLabelledBy === finalState.tabIds.find((tabId, index) => finalState.tabSelected[index] === "true") &&
    finalState.tabIndex.filter((tabIndex) => tabIndex === "0").length === 1;
  return {
    cardCount: finalState.cardCount,
    cardIds: cardResults.map(({ id }) => id),
    cardResults,
    allCardsVisible: cardResults.every((result) => result.visible),
    allCardsHitTestable: cardResults.every((result) => result.hitTestable),
    allCardsFocusable: cardResults.every((result) => result.focusable),
    everyCardSelectedAndRendersOneZone: cardResults.every((result) => result.selected && result.exactlyOneZone && result.zoneNames[0] === result.expectedZone),
    everyWorkbenchFullWidth: cardResults.every((result) => result.workbenchFullWidth),
    everyWorkbenchHasInternalScroll: cardResults.every((result) => result.bodyScrollable),
    everyWorkbenchHasStrictOverflowProbe: cardResults.every((result) => result.bodyOverflowProbePassed),
    everyWorkbenchOwnsOnlyScroll: cardResults.every((result) => result.nestedScrollportCount === 0 && result.nestedOverflowStyleCount === 0),
    activeId: finalState.selectedIds[0] ?? null,
    lastTabPointerHitTestable,
    dmxTabReachesWorkbench,
    keyboardNavigatesFromDmxToMidiAndDj:
      dmxPointerFocus &&
      arrowRightAudioState.selectedIds.length === 1 && arrowRightAudioState.selectedIds[0] === "audio" &&
      arrowRightState.selectedIds.length === 1 && arrowRightState.selectedIds[0] === "midi" &&
      endState.selectedIds.length === 1 && endState.selectedIds[0] === "dj",
    keyboardHomeReturnsToDmx:
      homeState.selectedIds.length === 1 && homeState.selectedIds[0] === "dmx",
    enterSelectsExactWorkbench: enterState.selectedIds.length === 1 && enterState.selectedIds[0] === "midi" && enterState.panelConnection === "midi",
    spaceSelectsExactWorkbench: spaceState.selectedIds.length === 1 && spaceState.selectedIds[0] === "osc" && spaceState.panelConnection === "osc",
    exactTabpanelRelation,
    activeWorkbenchLabelState: finalState.panelConnectionId === "dj" &&
      finalState.obsoleteHeaderCount === 0 &&
      finalState.selectedIds.length === 1 && finalState.selectedIds[0] === "dj",
    tabpanelRelationPerCard: cardResults.every((result) => result.tabpanelRelationExact),
    draftContinuity,
  };
}
