import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
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
  browser = await chromium.launch({headless: true});
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
  if (process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR) {
    await mkdir(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, {recursive: true});
    await writeFile(resolve(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, "fx-scroll-browser.json"), JSON.stringify({passed: true, results}, null, 2) + "\n");
  }
  console.log("FX form content and wheel/End save reachability: " + results.length + " passed");
} finally {
  await browser?.close();
  await server?.close();
}
