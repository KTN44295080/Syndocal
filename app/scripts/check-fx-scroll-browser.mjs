import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { mkdir, writeFile } from "node:fs/promises";
import { createServer } from "vite";

const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); }
catch { ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"))); }
let server, browser;
const results = [];
try {
  let url = process.env.SYNDOCAL_UI_TEST_URL;
  if (!url) {
    server = await createServer({root: resolve(import.meta.dirname, ".."), server: {host: "127.0.0.1", port: 0}});
    await server.listen();
    url = "http://127.0.0.1:" + server.httpServer.address().port;
  }
  // Prefer Playwright's bundled browser; fall back to an installed Chrome/Edge.
  const executablePath = process.env.SYNDOCAL_UI_TEST_BROWSER ?? [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].find(existsSync);
  try { browser = await chromium.launch({headless: true}); }
  catch (error) {
    if (!executablePath) throw error;
    browser = await chromium.launch({headless: true, executablePath});
  }
  const page = await browser.newPage();
  for (const viewport of [{width:1920,height:1080},{width:1920,height:1032},{width:2048,height:1152},{width:1366,height:768},{width:1280,height:720}]) {
    await page.setViewportSize(viewport);
    for (const family of ["COLOR FX", "CHASER FX", "VALUE FX", "CURVE FX", "2D MAPPING"]) {
      await page.goto(url + "/?syndocalViewportFixture=scene-matrix");
      await page.locator('[data-scene-matrix-edit-strip="301"]').click();
      await page.locator('[data-scene-settings-surface-control="fx"]').click();
      await page.locator('.effectFamilyChooser button').filter({hasText: family}).click();
      await page.locator('.sceneFxEditor .effectSubmitActions button.primary').waitFor();
      const measured = await page.locator('[data-scene-settings]').evaluate(root => {
        const editor = root.querySelector('.sceneFxEditor'), scroller = root.querySelector('.sceneSettingsScroller');
        return {fxHeight: editor.getBoundingClientRect().height, fxScrollHeight: editor.scrollHeight,
          scrollHeight: scroller.scrollHeight, clientHeight: scroller.clientHeight};
      });
      assert(measured.fxHeight + 2 >= measured.fxScrollHeight, family + " clips its form");
      const box = await page.locator('.sceneSettingsScroller').boundingBox();
      await page.mouse.move(box.x + box.width - 12, box.y + box.height / 2);
      await page.mouse.wheel(0, 900);
      await page.waitForFunction(() => document.querySelector('.sceneSettingsScroller').scrollTop > 0);
      await page.locator('.sceneSettingsScroller').focus();
      await page.keyboard.press('End');
      await page.waitForFunction(() => {
        const button = document.querySelector('.sceneFxEditor .effectSubmitActions button.primary');
        const b = button.getBoundingClientRect(), s = button.closest('.sceneSettingsScroller').getBoundingClientRect();
        return b.top >= s.top - 1 && b.bottom <= s.bottom + 1;
      });
      results.push({...viewport, family, ...measured, wheel: true, endSaveReachable: true});
    }
  }
  // Wide FX editing: the header toggle gives the pane most of the upper
  // workspace; the rack and editor sit side by side and the editor scrolls in
  // its own column. The preference persists and the toggle restores the rail.
  for (const viewport of [{width:1920,height:1080},{width:1366,height:768},{width:1280,height:720}]) {
    await page.setViewportSize(viewport);
    for (const family of ["COLOR FX", "CHASER FX", "2D MAPPING"]) {
      await page.goto(url + "/?syndocalViewportFixture=scene-matrix");
      await page.locator('[data-scene-matrix-edit-strip="301"]').click();
      await page.locator('[data-scene-settings-surface-control="fx"]').click();
      await page.locator('.effectFamilyChooser button').filter({hasText: family}).click();
      await page.locator('.sceneFxEditor .effectSubmitActions button.primary').waitFor();
      // After the first iteration the persisted preference reopens the pane
      // wide; narrow it first so both directions are exercised.
      const loadedWide = await page.locator('[data-scene-settings][data-scene-settings-wide="true"]').count() === 1;
      if (loadedWide) {
        await page.locator('[data-scene-settings-wide-toggle]').click();
        await page.waitForFunction(() => document.querySelector('[data-scene-settings]')?.getAttribute('data-scene-settings-wide') === null);
      }
      const narrowWidth = await page.locator('.sceneFxEditor').evaluate((editor) => editor.getBoundingClientRect().width);
      await page.locator('[data-scene-settings-wide-toggle]').click();
      await page.locator('[data-scene-settings][data-scene-settings-wide="true"]').waitFor();
      const wide = await page.locator('[data-scene-settings]').evaluate((root) => {
        const editor = root.querySelector('.sceneFxEditor').getBoundingClientRect();
        const chooser = root.querySelector('[data-scene-fx-chooser]').getBoundingClientRect();
        const pane = root.getBoundingClientRect();
        return {editorWidth: editor.width, editorTop: editor.top, chooserTop: chooser.top, chooserRight: chooser.right, editorLeft: editor.left,
          paneShare: pane.width / window.innerWidth, stored: window.localStorage.getItem("syndocal.sceneFxEditorWide.v1")};
      });
      assert(wide.editorWidth > narrowWidth + 100, family + " wide editor did not gain width");
      assert(Math.abs(wide.editorTop - wide.chooserTop) <= 2, family + " wide editor does not start at the top of the pane");
      assert(wide.editorLeft >= wide.chooserRight - 1, family + " wide editor overlaps the FX rack");
      assert(wide.paneShare > 0.55, family + " wide pane did not take most of the upper workspace");
      assert.equal(wide.stored, "1", family + " wide preference was not persisted");
      const editorBox = await page.locator('.sceneFxEditor').boundingBox();
      await page.mouse.move(editorBox.x + editorBox.width / 2, editorBox.y + editorBox.height / 2);
      for (let wheel = 0; wheel < 8; wheel += 1) {
        const reachable = await page.locator('.sceneFxEditor').evaluate((editor) => {
          const button = editor.querySelector('.effectSubmitActions button.primary').getBoundingClientRect();
          const box = editor.getBoundingClientRect();
          return button.top >= box.top - 1 && button.bottom <= box.bottom + 1;
        });
        if (reachable) break;
        await page.mouse.wheel(0, 600);
        await page.waitForTimeout(80);
      }
      await page.waitForFunction(() => {
        const editor = document.querySelector('.sceneFxEditor');
        const button = editor.querySelector('.effectSubmitActions button.primary').getBoundingClientRect();
        const box = editor.getBoundingClientRect();
        return button.top >= box.top - 1 && button.bottom <= box.bottom + 1;
      });
      results.push({...viewport, family, wide: true, loadedWide, narrowWidth, ...wide, wheelSaveReachable: true});
    }
  }
  assert(results.filter((result) => result.wide && result.loadedWide).length > 0, "wide FX preference did not survive a reload");
  await page.locator('[data-scene-settings-wide-toggle]').click();
  await page.waitForFunction(() => document.querySelector('[data-scene-settings]')?.getAttribute('data-scene-settings-wide') === null
    && window.localStorage.getItem("syndocal.sceneFxEditorWide.v1") === "0");
  if (process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR) {
    await mkdir(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, {recursive: true});
    await writeFile(resolve(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, "fx-scroll-browser.json"), JSON.stringify({passed: true, results}, null, 2) + "\n");
  }
  console.log("FX form content and wheel/End save reachability: " + results.length + " passed");
} finally {
  await browser?.close();
  await server?.close();
}
