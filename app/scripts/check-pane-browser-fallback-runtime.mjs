import assert from "node:assert/strict";

/**
 * Browser-only evidence for the pane-popup fallback. The caller supplies the
 * CDP client and its existing viewport/navigation helpers so this focused
 * proof cannot acquire an independent browser, server, or app fixture.
 */
export async function runPaneBrowserFallbackRuntimeProof({
  client,
  primaryOperationalViewport,
  fixtureUrl,
  navigateToReadyApp,
  navigateToReadyAppThroughExpectedBeforeUnload,
  clickWorkspaceOption,
  selectControlSurface,
  clickVisibleSelector,
  waitForClientCondition,
  sleep,
}) {
  const removeScenarioScript = async (identifier) => {
    // Preserve the assertion that caused a scenario failure. A navigation or
    // overloaded CDP target can make cleanup itself time out, but that must
    // not hide the trusted-input/runtime evidence from the caller.
    await client.send("Page.removeScriptToEvaluateOnNewDocument", { identifier }).catch(() => {});
  };
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: primaryOperationalViewport.width,
    height: primaryOperationalViewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const installScenario = async (mode) => client.send("Page.addScriptToEvaluateOnNewDocument", {
    source: `(() => {
      const fixture = {
        mode: ${JSON.stringify(mode)},
        failPaneStorage: ${mode === "storage-open" ? "true" : "false"},
        popupOpenCount: 0,
        popupCloseCount: 0,
        windowOpenUserActivations: [],
        normalNavigationCount: 0,
        popups: [],
      };
      window.__syndocalPaneBrowserFallbackFixture = fixture;
      const setItem = Storage.prototype.setItem;
      Storage.prototype.setItem = function(key, value) {
        if (key === "syndocal.paneWindows.v1" && fixture.failPaneStorage) {
          throw new Error("pane browser fallback storage fixture denied the write");
        }
        return setItem.call(this, key, value);
      };
      window.open = (url, target) => {
        // The popup object below is a deterministic lifecycle fixture, not a
        // simulation of browser popup policy. Every real fallback attempt is
        // still initiated by a trusted CDP mouse click, and this activation
        // sample fails if an await/import precedes window.open.
        fixture.windowOpenUserActivations.push({
          isActive: navigator.userActivation?.isActive ?? false,
          hasBeenActive: navigator.userActivation?.hasBeenActive ?? false,
        });
        fixture.popupOpenCount += 1;
        if (fixture.mode === "popup-null") return null;
        if (fixture.mode === "popup-throw") throw new Error("pane browser fallback popup fixture threw");
        let closed = false;
        let href = "about:blank";
        let documentReady = "loading";
        let paneRootMounted = false;
        const inspectionThrows = fixture.mode === "ready-inspection-error";
        const parsedTarget = new URL(url, window.location.href);
        const expectedPane = parsedTarget.searchParams.get("syndocalPaneWindow");
        const popup = {
          get closed() { return closed; },
          get location() {
            if (inspectionThrows) throw new Error("pane browser fallback popup inspection fixture threw");
            return { href };
          },
          get document() {
            if (inspectionThrows) throw new Error("pane browser fallback popup inspection fixture threw");
            return {
              get readyState() { return documentReady; },
              querySelector(selector) {
                return paneRootMounted && selector === '.app[data-pane-window-mode="' + expectedPane + '"]'
                  ? {}
                  : null;
              },
            };
          },
          target,
          requestedUrl: url,
          close() {
            fixture.popupCloseCount += 1;
            if (fixture.mode === "close-throw") {
              throw new Error("pane browser fallback popup close fixture threw");
            }
            if (fixture.mode === "close-still-open") return;
            closed = true;
          },
          navigate(nextUrl) {
            href = nextUrl;
            documentReady = "loading";
            paneRootMounted = false;
            fixture.normalNavigationCount += 1;
          },
          focus() {},
        };
        fixture.popups.push(popup);
        if (fixture.mode === "ready-close") {
          setTimeout(() => { closed = true; }, 25);
        } else if (fixture.mode !== "ready-timeout" && fixture.mode !== "ready-inspection-error") {
          setTimeout(() => {
            href = url;
            documentReady = "complete";
            paneRootMounted = true;
          }, 25);
        }
        return popup;
      };
    })();`,
  });
  const openWorkspacePane = async (pane) => {
    const dialogOpen = await client.evaluate("Boolean(document.querySelector('.workspaceOperationsPopover'))");
    if (!dialogOpen) {
      await clickVisibleSelector(client, ".workspaceOperationsButton");
      await waitForClientCondition(
        client,
        "Boolean(document.querySelector('.workspaceOperationsPopover'))",
        "workspace pane controls",
      );
    }
    const clicked = await client.evaluate(`(() => {
      const button = document.querySelector(
        '.workspaceOperationsPopover [data-workspace-pane-toggle=${JSON.stringify(pane)}]'
      );
      return button instanceof HTMLButtonElement && !button.disabled && (() => {
        const rect = button.getBoundingClientRect();
        const style = getComputedStyle(button);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      })();
    })()`);
    assert.equal(clicked, true, `browser fallback ${pane} control is available`);
    const point = await client.evaluate(`(() => {
      const button = document.querySelector(
        '.workspaceOperationsPopover [data-workspace-pane-toggle=${JSON.stringify(pane)}]'
      );
      if (!(button instanceof HTMLButtonElement) || button.disabled) return null;
      const rect = button.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    })()`);
    assert.ok(point, `browser fallback ${pane} control has a trusted click point`);
    await client.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: point.x,
      y: point.y,
      button: "left",
      clickCount: 1,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: point.x,
      y: point.y,
      button: "left",
      clickCount: 1,
    });
    await sleep(140);
  };
  const openWorkspacePaneRapidly = async (pane) => {
    const dialogOpen = await client.evaluate("Boolean(document.querySelector('.workspaceOperationsPopover'))");
    if (!dialogOpen) {
      await clickVisibleSelector(client, ".workspaceOperationsButton");
      await waitForClientCondition(
        client,
        "Boolean(document.querySelector('.workspaceOperationsPopover'))",
        "workspace pane controls",
      );
    }
    const clicked = await client.evaluate(`(() => {
      const button = document.querySelector(
        '.workspaceOperationsPopover [data-workspace-pane-toggle=${JSON.stringify(pane)}]'
      );
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      button.click();
      return true;
    })()`);
    assert.equal(clicked, true, `browser fallback rapid ${pane} controls are available`);
    await sleep(140);
  };
  const readState = (pane) => client.evaluate(`(() => {
    const content = document.querySelector(${JSON.stringify(
      pane === "stage" ? '[data-persistent-band-part="stage"]' : '[data-timeline-arranger-upper]',
    )});
    const band = document.querySelector('.mappingPersistentWorkspaceBand');
    const contentRect = content?.getBoundingClientRect();
    const contentStyle = content ? getComputedStyle(content) : null;
    const popups = window.__syndocalPaneBrowserFallbackFixture?.popups ?? [];
    const popup = popups.at(-1) ?? null;
    return {
      popped: Boolean(band?.classList.contains(${JSON.stringify(
        pane === "stage" ? "stagePanePopped" : "timelinePanePopped",
      )})),
      integrated: Boolean(
        contentRect && contentRect.width > 0 && contentRect.height > 0 &&
        contentStyle?.display !== 'none' && contentStyle.visibility !== 'hidden'
      ),
      storedPanes: window.localStorage.getItem('syndocal.paneWindows.v1'),
      status: document.querySelector('.appStatusText')?.textContent ?? '',
      popupOpenCount: window.__syndocalPaneBrowserFallbackFixture?.popupOpenCount ?? -1,
      popupCloseCount: window.__syndocalPaneBrowserFallbackFixture?.popupCloseCount ?? -1,
      normalNavigationCount: window.__syndocalPaneBrowserFallbackFixture?.normalNavigationCount ?? -1,
      windowOpenUserActivation: window.__syndocalPaneBrowserFallbackFixture?.windowOpenUserActivations?.at(-1) ?? null,
      popupClosed: (() => {
        try { return popup?.closed ?? null; } catch { return null; }
      })(),
      popupTarget: popup?.target ?? null,
      popupRequestedUrl: popup?.requestedUrl ?? null,
    };
  })()`);
  let hasNavigatedScenario = false;
  const startScenario = async (mode, poppedPane = null) => {
    const script = await installScenario(mode);
    try {
      const url = new URL(fixtureUrl("timeline"));
      if (poppedPane) url.searchParams.set("syndocalPoppedPanes", poppedPane);
      // The first navigation starts from the blank launch page and cannot
      // have a dirty App document. After a trusted pane click, however, the
      // real App beforeunload handler is eligible on the next scenario
      // navigation. Keep that dialog handling explicit and bounded: a missing
      // or wrong dialog fails closed instead of letting Page.navigate hang
      // until the generic CDP timeout.
      const navigate = hasNavigatedScenario
        ? navigateToReadyAppThroughExpectedBeforeUnload
        : navigateToReadyApp;
      await navigate(client, url.toString());
      hasNavigatedScenario = true;
      await clickWorkspaceOption(client, "control");
      await selectControlSurface(client, "live");
      await client.evaluate("window.localStorage.removeItem('syndocal.paneWindows.v1')");
      return script;
    } catch (error) {
      await removeScenarioScript(script.identifier);
      throw error;
    }
  };
  const assertIntegratedFailure = (state, visibleMessage, pane, label) => {
    assert.equal(state.popped, false, `${label}: no failed fallback may hide the main ${pane}`);
    assert.equal(state.integrated, true, `${label}: the main ${pane} stays integrated`);
    assert.match(state.status, visibleMessage, `${label}: the status line reports the actionable failure`);
  };
  for (const pane of ["stage", "timeline"]) {
    for (const [mode, message] of [
      ["popup-null", /Allow pop-ups for this site/],
      ["popup-throw", /could not open: Error: pane browser fallback popup fixture threw/],
      ["storage-open", /saving its browser window state failed/],
    ]) {
      const script = await startScenario(mode);
      try {
        await openWorkspacePane(pane);
        const state = await readState(pane);
        assert.equal(state.popupOpenCount, 1, `${pane} ${mode}: the fallback attempts exactly one popup`);
        assert.equal(state.windowOpenUserActivation?.isActive, true, `${pane} ${mode}: window.open runs inside trusted user activation`);
        assert.equal(state.storedPanes, null, `${pane} ${mode}: no detached pane set is persisted`);
        if (mode === "storage-open") {
          assert.equal(state.popupCloseCount, 1, `${pane} storage-open: the popup is retired`);
          assert.equal(state.popupClosed, true, `${pane} storage-open: the popup reports closed`);
        }
        assertIntegratedFailure(state, message, pane, `${pane} ${mode}`);
        // Prove both a synchronous failure (blocked popup) and an awaited
        // failure (ready popup whose persistence fails) release the per-pane
        // opening guard in this same mounted document. A remount would clear
        // the Set and could conceal a stranded guard.
        if (pane === "stage" && (mode === "popup-null" || mode === "storage-open")) {
          await client.evaluate(`(() => {
            const fixture = window.__syndocalPaneBrowserFallbackFixture;
            fixture.mode = "success";
            fixture.failPaneStorage = false;
          })()`);
          await openWorkspacePane(pane);
          const retried = await readState(pane);
          assert.equal(retried.popupOpenCount, 2, `${pane} ${mode}: same-document retry prepares exactly one new popup`);
          assert.equal(retried.popped, true, `${pane} ${mode}: same-document retry detaches after the guard is released`);
          assert.equal(retried.integrated, false, `${pane} ${mode}: successful retry leaves no duplicate main pane`);
          assert.equal(retried.storedPanes, '["stage"]', `${pane} ${mode}: successful retry persists the exact pane set`);
          await openWorkspacePane(pane);
          const rejoined = await readState(pane);
          assert.equal(
            rejoined.popupCloseCount,
            mode === "storage-open" ? 2 : 1,
            `${pane} ${mode}: retry and rejoin retire each created popup exactly once`,
          );
          assert.equal(rejoined.popped, false, `${pane} ${mode}: retry rejoin restores main content`);
          assert.equal(rejoined.integrated, true, `${pane} ${mode}: retry rejoin leaves the pane visible in main`);
          assert.equal(rejoined.storedPanes, "[]", `${pane} ${mode}: retry rejoin persists the exact empty set`);
        }
      } finally {
        await removeScenarioScript(script.identifier);
      }
    }
  }
  for (const [mode, message, expectedCloseCount] of [
    ["ready-close", /closed before its exact pane content became ready/, 0],
    ["ready-timeout", /did not become ready within 5000 ms/, 1],
    ["ready-inspection-error", /could not be inspected safely/, 1],
  ]) {
    const script = await startScenario(mode);
    try {
      await openWorkspacePane("stage");
      if (mode === "ready-timeout") await sleep(5_100);
      const state = await readState("stage");
      assert.equal(state.popupOpenCount, 1, `${mode}: one exact child is attempted`);
      assert.equal(state.popupCloseCount, expectedCloseCount, `${mode}: only a still-open unready child is retired`);
      if (mode !== "ready-inspection-error") {
        assert.equal(state.popupClosed, true, `${mode}: the unavailable child is closed or already closed`);
      }
      assert.equal(state.storedPanes, null, `${mode}: an unready child never persists detached state`);
      assertIntegratedFailure(state, message, "stage", mode);
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  {
    const script = await startScenario("success");
    try {
      await openWorkspacePaneRapidly("stage");
      const opened = await readState("stage");
      assert.equal(opened.popupOpenCount, 1, "browser concurrent opening: only one exact popup is prepared");
      assert.match(opened.status, /is already opening in the browser/, "browser concurrent opening: the second toggle fails visibly while readiness is pending");
      const openedUrl = new URL(opened.popupRequestedUrl, "http://localhost");
      const openedInstance = openedUrl.searchParams.get("syndocalPaneWindowInstance");
      assert.equal(openedUrl.searchParams.get("syndocalPaneWindow"), "stage", "browser success: child URL names the exact stage pane");
      assert.ok(openedInstance && openedInstance.startsWith("browser-"), "browser success: child URL carries a unique browser instance");
      assert.equal(opened.popupTarget, `syndocal-pane-stage-${openedInstance}`, "browser success: target is bound to the exact browser instance");
      assert.equal(opened.popped, true, "browser success: the pane hides only after persistence succeeds");
      assert.equal(opened.integrated, false, "browser success: the detached stage no longer renders in main");
      assert.equal(opened.storedPanes, '["stage"]', "browser success: the exact popped pane set is persisted");
      await client.evaluate("window.__syndocalPaneBrowserFallbackFixture.popups[0].navigate(window.__syndocalPaneBrowserFallbackFixture.popups[0].requestedUrl + '&normalNavigation=1')");
      await sleep(350);
      const navigated = await readState("stage");
      assert.equal(navigated.normalNavigationCount, 1, "normal navigation fixture executes once");
      assert.equal(navigated.popupClosed, false, "normal navigation does not close the retained popup");
      assert.equal(navigated.popped, true, "normal navigation is never misclassified as a rejoin");
      assert.equal(navigated.integrated, false, "normal navigation does not restore a duplicate main pane");
      await openWorkspacePane("stage");
      const rejoined = await readState("stage");
      assert.equal(rejoined.popupOpenCount, 1, "browser concurrent rejoin: no second child or orphan exists");
      assert.equal(rejoined.popupCloseCount, 1, "browser concurrent rejoin: the sole retained popup is closed once");
      assert.equal(rejoined.popupClosed, true, "browser concurrent rejoin: the sole child reports closed");
      assert.equal(rejoined.popped, false, "browser concurrent rejoin: the stage returns to main content");
      assert.equal(rejoined.integrated, true, "browser concurrent rejoin: the stage renders in main");
      assert.equal(rejoined.storedPanes, "[]", "browser concurrent rejoin: the exact empty set is persisted");
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  {
    const script = await startScenario("success");
    try {
      await openWorkspacePane("stage");
      await client.evaluate("window.__syndocalPaneBrowserFallbackFixture.failPaneStorage = true");
      await openWorkspacePane("stage");
      const state = await readState("stage");
      assert.equal(state.popupCloseCount, 1, "rejoin storage failure: the retained popup is still closed once");
      assert.equal(state.popupClosed, true, "rejoin storage failure: the retained popup reports closed");
      assertIntegratedFailure(state, /rejoined the main window, but clearing its browser window state failed/, "stage", "rejoin storage failure");
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  for (const mode of ["close-throw", "close-still-open"]) {
    const script = await startScenario(mode);
    try {
      await openWorkspacePane("stage");
      await openWorkspacePane("stage");
      const state = await readState("stage");
      assert.equal(state.popupCloseCount, 1, `${mode}: the retained popup receives one close attempt`);
      assert.equal(state.popupClosed, false, `${mode}: the failed popup retirement stays visible as open`);
      assertIntegratedFailure(state, /The browser popup could not close/, "stage", mode);
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  {
    const script = await startScenario("success");
    try {
      await openWorkspacePane("stage");
      await client.evaluate("window.__syndocalPaneBrowserFallbackFixture.popups[0].close()");
      await sleep(350);
      const state = await readState("stage");
      assert.equal(state.popupCloseCount, 1, "manual popup close: the popup close is observed exactly once");
      assert.equal(state.popupClosed, true, "manual popup close: the retained popup reports closed");
      assert.equal(state.popped, false, "manual popup close: the main stage is immediately reintegrated");
      assert.equal(state.integrated, true, "manual popup close: the stage renders in main");
      assert.match(state.status, /was closed in the browser and rejoined the main window/, "manual popup close is visible");
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  {
    const script = await startScenario("success", "stage");
    try {
      await openWorkspacePane("stage");
      const state = await readState("stage");
      assert.equal(state.popupOpenCount, 0, "missing-handle rejoin: no popup was minted for URL-seeded state");
      assertIntegratedFailure(state, /The browser popup handle was missing/, "stage", "missing-handle rejoin");
    } finally {
      await removeScenarioScript(script.identifier);
    }
  }
  return { passed: true };
}
