import assert from "node:assert/strict";

export async function assertTimelineSourceTargetReveal({ client, click, evaluate, waitFor }) {
  assert.equal(
    await click(client, '[data-timeline-source-shelf-category="media"]'),
    true,
    "1280 Timeline Sources opens Media Library target controls",
  );
  await waitFor(
    () => evaluate(client, "document.querySelector('#timeline-source-context-panel-media')?.getBoundingClientRect().height > 0"),
    "1280 Timeline Media Library source panel",
  );
  assert.equal(
    await click(client, '#timeline-source-context-panel-media .timelineExternalSourcePlacementDisclosure > summary'),
    true,
    "1280 Timeline Media Library target disclosure opens",
  );
  for (const kind of ["Video", "Audio"]) {
    const prepared = await evaluate(client, `(() => {
      const scrollport = document.querySelector('[data-timeline-layer-scrollport]');
      const select = document.querySelector('[data-timeline-source-target-kind="${kind}"]');
      const app = document.querySelector('.app');
      if (!(scrollport instanceof HTMLElement) || !(select instanceof HTMLSelectElement)) return null;
      const options = [...select.options].filter((candidate) => Number(candidate.value) > 0);
      const extremes = [0, Math.max(0, scrollport.scrollHeight - scrollport.clientHeight)];
      let option = null;
      let initialScrollTop = 0;
      for (const candidate of options) {
        const target = scrollport.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="' + candidate.value + '"]');
        if (!(target instanceof HTMLElement)) continue;
        for (const extreme of extremes) {
          scrollport.scrollTop = extreme;
          const portRect = scrollport.getBoundingClientRect();
          const targetRect = target.getBoundingClientRect();
          if (targetRect.top < portRect.top - 1 || targetRect.bottom > portRect.bottom + 1) {
            option = candidate;
            initialScrollTop = scrollport.scrollTop;
            break;
          }
        }
        if (option) break;
      }
      if (!(option instanceof HTMLOptionElement)) return null;
      scrollport.scrollTop = initialScrollTop;
      select.focus();
      const before = {
        documentTop: document.documentElement.scrollTop,
        documentLeft: document.documentElement.scrollLeft,
        appTop: app instanceof HTMLElement ? app.scrollTop : null,
        appLeft: app instanceof HTMLElement ? app.scrollLeft : null,
        layerTop: scrollport.scrollTop,
      };
      select.value = option.value;
      select.dispatchEvent(new Event('change', { bubbles: true }));
      return { targetId: Number(option.value), before };
    })()`);
    assert.ok(prepared?.targetId > 0, `1280 ${kind} source target has a selectable off-screen lane`);
    const proof = await waitFor(() => evaluate(client, `(() => {
      const scrollport = document.querySelector('[data-timeline-layer-scrollport]');
      const target = scrollport?.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="${prepared.targetId}"]');
      const select = document.querySelector('[data-timeline-source-target-kind="${kind}"]');
      const app = document.querySelector('.app');
      if (!(scrollport instanceof HTMLElement) || !(target instanceof HTMLElement) || !(select instanceof HTMLSelectElement)) return null;
      const portRect = scrollport.getBoundingClientRect();
      const targetRect = target.getBoundingClientRect();
      const result = {
        targetId: Number(select.value),
        layerTop: scrollport.scrollTop,
        visible: targetRect.top >= portRect.top - 1 && targetRect.bottom <= portRect.bottom + 1,
        focused: document.activeElement === select,
        documentTop: document.documentElement.scrollTop,
        documentLeft: document.documentElement.scrollLeft,
        appTop: app instanceof HTMLElement ? app.scrollTop : null,
        appLeft: app instanceof HTMLElement ? app.scrollLeft : null,
      };
      return Math.abs(result.layerTop - ${prepared.before.layerTop}) > 0.5 && result.visible ? result : null;
    })()`), `1280 ${kind} target lane reveal`);
    assert.deepEqual(
      {
        targetId: proof.targetId,
        focused: proof.focused,
        documentTop: proof.documentTop,
        documentLeft: proof.documentLeft,
        appTop: proof.appTop,
        appLeft: proof.appLeft,
      },
      {
        targetId: prepared.targetId,
        focused: true,
        documentTop: prepared.before.documentTop,
        documentLeft: prepared.before.documentLeft,
        appTop: prepared.before.appTop,
        appLeft: prepared.before.appLeft,
      },
      `1280 ${kind} target selection reveals only its Timeline lane and preserves selector focus/page scroll`,
    );
  }
  await evaluate(client, `(() => {
    const scrollport = document.querySelector('[data-timeline-layer-scrollport]');
    if (scrollport instanceof HTMLElement) scrollport.scrollTop = 0;
  })()`);
  assert.equal(
    await click(client, '[data-timeline-source-shelf-category="scenes"]'),
    true,
    "1280 Timeline Sources restores Scenes after target-reveal proof",
  );
}
