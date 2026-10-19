// Browser proof for the in-app confirmation dialog on the real application shell: a modal
// alertdialog in every window (main and pop-out pane), danger dialogs start on Cancel, Escape
// and Enter-on-Cancel decline, focus is trapped and restored, shortcuts are suspended, and no
// WebView2 browser dialog ("tauri.localhost says...") ever opens. Software only; no native I/O.
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),
    ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifacts = resolve(process.env.SYNDOCAL_DIALOG_ARTIFACT_DIR ?? resolve(appRoot, "../target/qa/task14/in-app-dialog"));
const executablePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
await mkdir(artifacts, { recursive: true });

const server = await createServer({ root: appRoot, cacheDir: resolve(artifacts, "vite-cache"), logLevel: "error", server: { host: "127.0.0.1", port: Number(process.env.SYNDOCAL_DIALOG_PORT ?? 5283), strictPort: true } });
await server.listen();
const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
const browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
const nativeDialogs = [];
const errors = [];
const dialog = "[data-in-app-dialog][open]";
try {
  const context = await browser.newContext({ viewport: { width: 1366, height: 768 } });
  const open = async (url) => {
    const page = await context.newPage();
    page.on("dialog", (d) => { if (d.type() === "beforeunload") void d.accept(); else { nativeDialogs.push({ type: d.type(), message: d.message() }); void d.dismiss(); } });
    page.on("pageerror", (error) => errors.push(String(error)));
    for (let attempt = 0; attempt < 4; attempt++) {
      try { await page.goto(url, { timeout: 120_000 }); await page.locator(".app").waitFor({ timeout: 120_000 }); break; } catch (error) { if (attempt === 3) throw error; await page.waitForTimeout(2000); }
    }
    return page;
  };
  const settle = (page) => page.evaluate(async () => { await document.fonts.ready; await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))); });
  const state = (page) => page.evaluate((selector) => {
    const d = document.querySelector(selector);
    if (!d) return null;
    const title = document.getElementById(d.getAttribute("aria-labelledby"));
    const detail = document.getElementById(d.getAttribute("aria-describedby"));
    const active = document.activeElement;
    const rect = d.getBoundingClientRect();
    return {
      role: d.getAttribute("role"), ariaModal: d.getAttribute("aria-modal"), modal: d.matches(":modal"), tone: d.dataset.tone,
      title: title?.innerText ?? "", detail: detail?.innerText ?? "",
      focusOnCancel: Boolean(active?.matches("[data-in-app-dialog-cancel]")), focusInside: d.contains(active),
      inViewport: rect.left >= 0 && rect.top >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight,
      buttons: [...d.querySelectorAll("button")].map((b) => ({ text: b.innerText.trim(), width: Math.round(b.getBoundingClientRect().width), height: Math.round(b.getBoundingClientRect().height) })),
    };
  }, dialog);

  // The same assertions for every window: the main window uses the real fixture Remove control; a
  // pop-out pane window has no such control on screen, so it asks the same in-app question through the
  // shared service (the module instance the application itself uses) and must draw its own modal.
  for (const target of [
    { label: "main", url: `${origin}/?syndocalViewportFixture=patch`, real: true },
    { label: "pane-setup", url: `${origin}/?syndocalViewportFixture=patch&syndocalPaneWindow=setup`, real: false },
    { label: "pane-stage", url: `${origin}/?syndocalViewportFixture=patch&syndocalPaneWindow=stage`, real: false },
    { label: "pane-timeline", url: `${origin}/?syndocalViewportFixture=timeline&syndocalPaneWindow=timeline`, real: false },
  ]) {
    const page = await open(target.url);
    await settle(page);
    if (target.label === "main") await page.locator('[data-workspace-option="setup"]').click();
    await settle(page);
    let trigger;
    let ask;
    if (target.real) {
      // Trigger: the selection panel's Remove (fixtures) question.
      trigger = page.locator(".mappingSelectionPanel button", { hasText: /^\s*Remove/ }).first();
      assert.ok(await trigger.count() > 0, `${target.label}: the fixture Remove control is reachable`);
      await trigger.focus();
      ask = async () => { await trigger.click(); };
    } else {
      trigger = page.locator("button:visible").first();
      assert.ok(await trigger.count() > 0, `${target.label}: the pane window shows a control`);
      await trigger.focus();
      ask = async () => {
        await trigger.focus();
        await page.evaluate(async () => {
          const service = await import("/src/inAppDialog.ts");
          window.__dialogAnswer = "pending";
          service.confirmDialog({ title: 'Remove fixture "Pane test"?', message: "You can undo this from the Project menu or with Ctrl/Cmd+Z.", confirmLabel: "Remove", tone: "danger" })
            .then((answer) => { window.__dialogAnswer = answer; });
        });
      };
    }
    await ask();
    await page.waitForSelector(dialog, { timeout: 5000 });
    const shown = await state(page);
    assert.equal(shown.role, "alertdialog", `${target.label}: role`);
    assert.ok(shown.modal && shown.ariaModal === "true", `${target.label}: modal`);
    assert.equal(shown.tone, "danger", `${target.label}: removing a fixture is a danger question`);
    assert.ok(shown.focusOnCancel, `${target.label}: a danger dialog starts on Cancel`);
    assert.match(shown.title, /Remove fixture "[^"]+"\?|Remove \d+ selected fixtures\?/, `${target.label}: title names the fixture`);
    assert.match(shown.detail, /undo/i, `${target.label}: message names the consequence`);
    assert.ok(shown.inViewport, `${target.label}: dialog fits the viewport`);
    assert.ok(shown.buttons.length === 2 && shown.buttons.every((b) => b.height >= 44 && b.width >= 44), `${target.label}: 44px targets ${JSON.stringify(shown.buttons)}`);
    await page.screenshot({ path: resolve(artifacts, `${target.label}-dialog.png`) });
    // Shortcuts are suspended: a global Ctrl+N must not open anything else; Tab stays inside.
    await page.keyboard.press("Control+KeyN");
    for (let i = 0; i < 4; i++) { await page.keyboard.press("Tab"); assert.ok(await page.evaluate(() => document.activeElement?.closest("[data-in-app-dialog]") !== null), `${target.label}: Tab stays inside`); }
    await page.keyboard.press("Shift+Tab");
    assert.ok(await page.evaluate(() => document.activeElement?.closest("[data-in-app-dialog]") !== null), `${target.label}: Shift+Tab stays inside`);
    assert.equal(await page.locator(dialog).count(), 1, `${target.label}: exactly one dialog`);
    // The page behind a modal is inert: clicking it does nothing.
    await page.mouse.click(20, 400).catch(() => {});
    assert.equal(await page.locator(dialog).count(), 1, `${target.label}: the dialog survives a click behind it`);
    // Escape declines and focus returns to the trigger.
    await page.keyboard.press("Escape");
    await page.waitForFunction((selector) => !document.querySelector(selector), dialog);
    assert.ok(await trigger.evaluate((el) => el === document.activeElement), `${target.label}: focus returns to the trigger after Escape`);
    if (!target.real) assert.equal(await page.evaluate(() => window.__dialogAnswer), false, `${target.label}: Escape answers false`);
    // Enter on Cancel is harmless.
    await ask();
    await page.waitForSelector(dialog);
    assert.ok((await state(page)).focusOnCancel, `${target.label}: Cancel focused again`);
    await page.keyboard.press("Enter");
    await page.waitForFunction((selector) => !document.querySelector(selector), dialog);
    assert.ok(await trigger.evaluate((el) => el === document.activeElement), `${target.label}: focus returns after Enter on Cancel`);
    if (!target.real) assert.equal(await page.evaluate(() => window.__dialogAnswer), false, `${target.label}: Enter on Cancel answers false`);
    // Confirm closes the dialog (the browser preview has no desktop backend to act).
    await ask();
    await page.waitForSelector(dialog);
    await page.locator("[data-in-app-dialog-confirm]").click();
    await page.waitForFunction((selector) => !document.querySelector(selector), dialog);
    if (!target.real) assert.equal(await page.evaluate(() => window.__dialogAnswer), true, `${target.label}: Confirm answers true`);
    assert.equal(nativeDialogs.length, 0, `${target.label}: no browser dialog opened ${JSON.stringify(nativeDialogs)}`);
    await page.close();
    console.log(`PASS ${target.label} window`);
  }
  assert.deepEqual(errors, [], "no runtime exceptions");
  console.log(`In-app dialog browser proof: main and pop-out windows; screenshots in ${artifacts}`);
} finally {
  await browser.close();
  await server.close();
}
