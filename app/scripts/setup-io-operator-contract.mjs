/* Focused browser contract for the connection-first Setup I/O operator deck. */

const expectedZoneByConnection = {
  dmx: "dmx",
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
        panelHeaderLabel: panel?.querySelector(":scope > .setupIoWorkbenchHeader h2")?.textContent?.trim() ?? null,
        panelSummaryCount: panel?.querySelectorAll(":scope > .setupIoWorkbenchHeader p").length ?? 0,
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
  let fifthTabPointerHitTestable = false;
  for (const id of Object.keys(expectedZoneByConnection)) {
    const geometry = await clickIoConnectionTab(client, id);
    if (id === "dj") fifthTabPointerHitTestable = geometry.hitTestable;
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

  await selectIoConnection("dj");
  const activeBeforeQuickAction = await evaluatePageFunction(client, () =>
    document.querySelector('[data-io-primary-connection] > button[role="tab"][aria-selected="true"]')
      ?.closest("[data-io-primary-connection]")?.getAttribute("data-io-primary-connection") ?? null,
  );
  const quickActionGeometry = await client.evaluate(`(() => {
    const action = document.querySelector('[data-io-primary-connection="dmx"] > .setupIoConnectionMeta > .setupIoConnectionAction > button');
    if (!(action instanceof HTMLElement)) return { found: false, hitTestable: false };
    const rect = action.getBoundingClientRect();
    const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return {
      found: rect.width > 0 && rect.height > 0,
      hitTestable: target === action || action.contains(target),
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
    };
  })()`);
  if (!quickActionGeometry.found || !quickActionGeometry.hitTestable) {
    throw new Error(`Setup I/O inactive quick action is not hit-testable: ${JSON.stringify(quickActionGeometry)}`);
  }
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: quickActionGeometry.x, y: quickActionGeometry.y });
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed", x: quickActionGeometry.x, y: quickActionGeometry.y,
    button: "left", buttons: 1, clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased", x: quickActionGeometry.x, y: quickActionGeometry.y,
    button: "left", buttons: 0, clickCount: 1,
  });
  await sleep(80);
  const activeAfterQuickAction = await evaluatePageFunction(client, () =>
    document.querySelector('[data-io-primary-connection] > button[role="tab"][aria-selected="true"]')
      ?.closest("[data-io-primary-connection]")?.getAttribute("data-io-primary-connection") ?? null,
  );

  await selectIoConnection("dmx");
  const dmxPointerFocus = await evaluatePageFunction(client, () => {
    const button = document.querySelector('[data-io-primary-connection="dmx"] > button[role="tab"]');
    return button instanceof HTMLElement && document.activeElement === button;
  });
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
    fifthTabPointerHitTestable,
    inactiveQuickActionPreservesSelection: activeBeforeQuickAction === "dj" && activeAfterQuickAction === "dj",
    keyboardNavigatesFromDmxToMidiAndDj:
      dmxPointerFocus &&
      arrowRightState.selectedIds.length === 1 && arrowRightState.selectedIds[0] === "midi" &&
      endState.selectedIds.length === 1 && endState.selectedIds[0] === "dj",
    keyboardHomeReturnsToDmx:
      homeState.selectedIds.length === 1 && homeState.selectedIds[0] === "dmx",
    enterSelectsExactWorkbench: enterState.selectedIds.length === 1 && enterState.selectedIds[0] === "midi" && enterState.panelConnection === "midi",
    spaceSelectsExactWorkbench: spaceState.selectedIds.length === 1 && spaceState.selectedIds[0] === "osc" && spaceState.panelConnection === "osc",
    exactTabpanelRelation,
    activeWorkbenchLabelState: finalState.panelHeaderLabel === "DJ Link" && finalState.panelSummaryCount === 0,
    tabpanelRelationPerCard: cardResults.every((result) => result.tabpanelRelationExact),
  };
}
