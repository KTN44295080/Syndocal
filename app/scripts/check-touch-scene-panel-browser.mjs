import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

// Real App in the browser scene-matrix fixture: Control > Lighting shows every
// Scene grouped by Bank. No native backend, device or output is involved.
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifactDir = resolve(process.env.SYNDOCAL_TOUCH_SCENE_ARTIFACT_DIR ?? resolve(root, "../target/qa/touch-scene-panel"));
const port = Number(process.env.SYNDOCAL_TOUCH_SCENE_PORT ?? 5183);
await mkdir(artifactDir, { recursive: true });
let server, browser;
try {
  server = await createServer({ root, server: { host: "127.0.0.1", port, strictPort: true }, logLevel: "error" });
  await server.listen();
  const origin = `http://127.0.0.1:${port}/?syndocalViewportFixture=scene-matrix`;
  const executablePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  const evidence = { timestamp: new Date().toISOString(), browserVersion: browser.version(), cases: [] };
  for (const locale of ["en", "ja"]) {
    for (const [width, height] of [[1920, 1080], [960, 640]]) {
      const label = `${locale}-${width}x${height}`;
      const context = await browser.newContext({ viewport: { width, height } });
      await context.addInitScript(value => { try { localStorage.setItem("syndocal.uiLocale.v1", value); } catch {} }, locale);
      const page = await context.newPage();
      const errors = [];
      page.on("pageerror", error => errors.push(String(error)));
      await page.goto(origin);
      await page.locator('[data-workspace-option="touch"]').click({ timeout: 60_000 });
      await page.locator('[data-control-domain="lighting"]').click();
      await page.locator("[data-touch-scene-panel]").waitFor({ timeout: 30_000 });
      await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
      const measure = () => page.evaluate(() => {
        const rect = el => { const r = el.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom }; };
        const snapshot = window.__syndocalReadSceneMatrixFixtureSnapshot();
        const panel = document.querySelector("[data-touch-scene-panel]");
        const grid = document.querySelector(".touchSurfaceGrid");
        const gridStyle = getComputedStyle(grid), g = grid.getBoundingClientRect();
        const scrollport = { x: g.x + parseFloat(gridStyle.borderLeftWidth), y: g.y + parseFloat(gridStyle.borderTopWidth) };
        scrollport.right = scrollport.x + grid.clientWidth; scrollport.bottom = scrollport.y + grid.clientHeight;
        const p = panel?.getBoundingClientRect();
        const tiles = [...document.querySelectorAll("[data-touch-control]")].map(el => ({ id: el.dataset.touchControl, ...rect(el) }));
        const overlapping = p ? tiles.filter(t => Math.min(t.right, p.right) - Math.max(t.x, p.left) > 1 && Math.min(t.bottom, p.bottom) - Math.max(t.y, p.top) > 1)
          .filter(t => { const pr = grid.getBoundingClientRect(); return t.bottom > pr.top && t.y < pr.bottom; }) : [];
        const buttons = [...document.querySelectorAll("[data-touch-scene]")].map(el => ({ id: Number(el.dataset.touchScene), label: el.querySelector("strong")?.textContent,
          ...rect(el), fontSize: parseFloat(getComputedStyle(el).fontSize), pressed: el.getAttribute("aria-pressed"), flash: el.classList.contains("flash") }));
        const banks = [...document.querySelectorAll("[data-touch-scene-bank]")].map(el => ({ id: Number(el.dataset.touchSceneBank),
          scenes: [...el.querySelectorAll("[data-touch-scene]")].map(b => Number(b.dataset.touchScene)) }));
        const toggle = document.querySelector("[data-touch-scene-toggle]");
        return { snapshotCues: snapshot.cues.map(c => ({ id: c.id, bank: c.cue_list_id })), cueLists: snapshot.cue_lists?.map(l => l.id) ?? null,
          panel: p ? rect(panel) : null, scrollport, overlapping, tiles, buttons, banks,
          toggle: toggle ? { text: toggle.textContent.trim(), pressed: toggle.getAttribute("aria-pressed"), ...rect(toggle) } : null,
          pageScroll: { x: document.documentElement.scrollWidth - innerWidth, y: document.documentElement.scrollHeight - innerHeight },
          lang: document.documentElement.lang };
      });
      const open = await measure();
      await page.screenshot({ path: resolve(artifactDir, `after-${label}.png`) });
      assert.equal(open.lang, locale, `${label}: locale applied`);
      assert.ok(open.panel, `${label}: Scene panel is shown by default on the Default Desk`);
      assert.equal(open.toggle?.text, locale === "ja" ? "シーン" : "Scenes", `${label}: Scene toggle label`);
      assert.ok(open.toggle.w >= 48 && open.toggle.h >= 48, `${label}: toggle keeps the 48px Touch target`);
      assert.ok(open.snapshotCues.length >= 5, `${label}: fixture has more than four Scenes`);
      assert.deepEqual(open.buttons.map(b => b.id).sort((a, b) => a - b), open.snapshotCues.map(c => c.id).sort((a, b) => a - b), `${label}: every Scene is present once`);
      for (const bank of open.banks) for (const id of bank.scenes)
        assert.equal(open.snapshotCues.find(c => c.id === id).bank, bank.id, `${label}: Scene ${id} sits in its own Bank`);
      assert.deepEqual(open.overlapping, [], `${label}: panel covers no Default Desk control`);
      const p = open.panel, s = open.scrollport;
      assert.ok(p.x >= s.x - 1 && p.right <= s.right + 1 && p.y >= s.y - 1 && p.bottom <= s.bottom + 1, `${label}: panel stays inside the grid scrollport ${JSON.stringify({ p, s })}`);
      assert.ok(p.right <= width && p.bottom <= height, `${label}: panel inside the viewport`);
      assert.deepEqual(open.pageScroll, { x: 0, y: 0 }, `${label}: no page scroll`);
      assert.ok(open.buttons.every(b => b.h >= 48 && b.w >= 48 && b.fontSize >= 13), `${label}: Scene buttons keep 48px targets and text size`);
      // Every Scene is reachable by scrolling inside the panel and is hit-testable.
      const reach = await page.evaluate(async () => {
        const out = [];
        for (const el of document.querySelectorAll("[data-touch-scene]")) {
          el.scrollIntoView({ block: "nearest" });
          await new Promise(r => requestAnimationFrame(r));
          const r = el.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
          out.push(el.contains(hit));
        }
        document.querySelector("[data-touch-scene-panel]").scrollTop = 0;
        return { all: out.every(Boolean), pageScrollY: window.scrollY, appScroll: document.querySelector(".app")?.scrollTop ?? 0 };
      });
      assert.deepEqual(reach, { all: true, pageScrollY: 0, appScroll: 0 }, `${label}: every Scene reachable by internal scrolling`);
      // Grid tracks are identical with the panel open or closed: nothing shrinks.
      await page.locator("[data-touch-scene-toggle]").click();
      const closed = await measure();
      assert.equal(closed.panel, null, `${label}: toggle hides the panel`);
      assert.equal(closed.toggle.pressed, "false");
      assert.deepEqual(closed.tiles, open.tiles, `${label}: Touch tiles are unchanged by the panel`);
      await page.locator("[data-touch-scene-toggle]").click();
      await page.locator("[data-touch-scene-panel]").waitFor();
      // Fifth or later Scene: one click fires it with no workspace switch; a second click releases it.
      const fifth = open.buttons.slice(4).find(button => !button.flash);
      const target = page.locator(`[data-touch-scene="${fifth.id}"]`);
      await target.scrollIntoViewIfNeeded();
      await target.click();
      const fired = await page.evaluate(id => {
        const s = window.__syndocalReadSceneMatrixFixtureSnapshot(), cue = s.cues.find(c => c.id === id);
        return { active: cue.group_id ? s.active_group_cue_ids?.[cue.group_id] === id : s.active_cue_id === id,
          pressed: document.querySelector(`[data-touch-scene="${id}"]`).getAttribute("aria-pressed"),
          workspace: document.querySelector('[data-workspace-option="touch"]').getAttribute("aria-pressed") };
      }, fifth.id);
      assert.deepEqual(fired, { active: true, pressed: "true", workspace: "true" }, `${label}: fifth Scene fires in one click on Control`);
      await target.click();
      assert.equal(await target.getAttribute("aria-pressed"), "false", `${label}: pressing the active Scene releases it`);
      await target.click();
      await page.screenshot({ path: resolve(artifactDir, `after-${label}-fired.png`) });
      await target.click();
      // Flash Scenes stay momentary: active while held, released on pointer up.
      const flash = open.buttons.find(button => button.flash);
      if (flash) {
        const flashButton = page.locator(`[data-touch-scene="${flash.id}"]`);
        await flashButton.scrollIntoViewIfNeeded();
        const box = await flashButton.boundingBox();
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.down();
        const held = await page.evaluate(id => window.__syndocalReadSceneMatrixFixtureSnapshot().active_group_cue_ids?.back === id
          || window.__syndocalReadSceneMatrixFixtureSnapshot().active_cue_id === id, flash.id);
        await page.mouse.up();
        const releasedFlash = await page.evaluate(id => window.__syndocalReadSceneMatrixFixtureSnapshot().active_cue_id !== id, flash.id);
        assert.deepEqual({ held, releasedFlash }, { held: true, releasedFlash: true }, `${label}: Flash Scene is momentary`);
      }
      // The operator's choice persists on this device.
      await page.locator("[data-touch-scene-toggle]").click();
      await page.reload();
      await page.locator('[data-workspace-option="touch"]').click({ timeout: 60_000 });
      await page.locator('[data-control-domain="lighting"]').click();
      await page.locator("[data-touch-scene-toggle]").waitFor();
      assert.equal(await page.locator("[data-touch-scene-panel]").count(), 0, `${label}: closed choice persists`);
      assert.deepEqual(errors, [], `${label}: page errors`);
      evidence.cases.push({ label, open: { ...open, tiles: open.tiles.length }, fired, reach, clicksForFifthScene: 1 });
      await context.close();
    }
  }
  await writeFile(resolve(artifactDir, "touch-scene-evidence.json"), JSON.stringify(evidence, null, 2) + "\n");
  console.log(`PASS Control > Lighting Scene panel: ${evidence.cases.length} cases (en/ja x 1920x1080/960x640), every Scene by Bank, inside the grid scrollport over free columns, unchanged tiles, 48px targets, 1-click fire/release, persisted toggle; evidence=${artifactDir}`);
} finally {
  await browser?.close();
  await server?.close();
}
