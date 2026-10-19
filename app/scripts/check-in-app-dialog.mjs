// In-app dialog service and wording contract: one question at a time, declining
// is always the safe answer, and every dialog string has Japanese text.
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cacheDir = await mkdtemp(join(tmpdir(), "syndocal-in-app-dialog-"));
const server = await createServer({
  root: appRoot, cacheDir, appType: "custom", logLevel: "error", clearScreen: false,
  server: { middlewareMode: true }, ssr: { noExternal: ["solid-js"] }, optimizeDeps: { noDiscovery: true, include: [] },
});
try {
  const service = await server.ssrLoadModule("/src/inAppDialog.ts");
  const copy = await server.ssrLoadModule("/src/inAppDialogCopy.ts");
  const destructive = await server.ssrLoadModule("/src/destructiveActions.ts");
  const localization = await server.ssrLoadModule("/src/uiLocalization.ts");
  const open = () => service.activeInAppDialog();
  const tick = () => new Promise((done) => setTimeout(done, 0));

  // Confirm: only an explicit confirm resolves true; the dialog is gone afterwards.
  let answer = service.confirmDialog({ title: "Remove it?", message: "It is gone.", tone: "danger" });
  assert.equal(open().kind, "confirm");
  assert.equal(open().tone, "danger");
  assert.deepEqual(open().message, ["It is gone."], "a single message becomes one paragraph");
  assert.equal(open().confirmLabel, "OK");
  assert.equal(open().cancelLabel, "Cancel");
  assert.equal(service.isInAppDialogOpen(), true);
  open().settle(true);
  assert.equal(await answer, true);
  assert.equal(open(), null);
  assert.equal(service.isInAppDialogOpen(), false);

  // Cancel (and Escape, which the host maps to settle(false)) is false.
  answer = service.confirmDialog({ title: "Go?", message: ["One", "Two"] });
  assert.equal(open().tone, "default");
  assert.deepEqual(open().message, ["One", "Two"]);
  open().settle(false);
  assert.equal(await answer, false);

  // One question at a time: a second confirmation never queues; it declines at once and
  // leaves the first question untouched, so no stale or duplicate prompt can confirm.
  answer = service.confirmDialog({ title: "First", message: "a" });
  const first = open();
  assert.equal(await service.confirmDialog({ title: "Second", message: "b" }), false, "a second question declines immediately");
  assert.equal(open(), first, "the first question stays up");
  // A dialog settles once: a late second click cannot touch its successor.
  first.settle(false);
  assert.equal(await answer, false);
  const next = service.confirmDialog({ title: "Next", message: "c" });
  const successor = open();
  first.settle(true);
  assert.equal(open(), successor, "settling a closed dialog does not close its successor");
  successor.settle(true);
  assert.equal(await next, true);

  // Alerts wait for the dialog in front of them and have no cancel.
  const blocker = service.confirmDialog({ title: "Blocker", message: "x" });
  let alertShown = false;
  const alerted = service.alertDialog({ title: "Heads up", message: "Read this." }).then(() => { alertShown = true; });
  await tick();
  assert.equal(open().kind, "confirm", "the alert is queued behind the open question");
  open().settle(false);
  await blocker;
  for (let i = 0; i < 40 && open()?.kind !== "alert"; i++) await new Promise((done) => setTimeout(done, 25));
  assert.equal(open().kind, "alert");
  assert.equal(open().cancelLabel, null, "an alert has no way to decline");
  assert.equal(alertShown, false);
  open().settle(true);
  await alerted;
  assert.equal(alertShown, true);
  assert.equal(open(), null);

  // Wording: every string a dialog can show has Japanese text, and every
  // dangerous question names its consequence and starts on Cancel.
  const samples = [
    copy.blackoutDialog(true), copy.blackoutDialog(false), copy.newProjectDuringShowDialog(),
    copy.discardTimelineEditsDialog(), copy.discardFxDraftDialog(), copy.installUpdateDialog("1.2.3"),
    copy.removeFixturesDialog(1, "Front Spot 1"), copy.removeFixturesDialog(3, "x"), copy.abandonPublicationDialog("save as"),
    copy.abandonPublicationDialog("user template"), copy.deleteBackupDialog("2026/10/19 12:00:00"), copy.disarmDjLinkDialog(),
    copy.rotateDjLinkTokenDialog(), copy.returnDjControlDialog(), copy.adoptMediaReplacementDialog(7, "C:\\media\\a.mp4"),
    copy.revokeAllExternalAccessDialog(), copy.removeTimelineDialog("Timeline 2"), copy.deleteTouchPageDialog("Page 2"),
    copy.removeTouchControlDialog("Fader 1"), copy.removeDjLinkMappingDialog(), copy.removeClipSlotDialog(12),
    copy.removeFxPresetDialog("Warm"), copy.removeLayerGroupDialog("Front"), copy.removeFxStageDialog("Invert"),
    copy.removeFxChainDialog("Layer 1"), copy.removeTransitionBusDialog("Bus A"), copy.addEffectToLiveLayerDialog("3 · Logo", "Mirror"),
    destructive.destructiveActionDialog("fixture", "Front Spot 1"), destructive.destructiveActionDialog("playback executor", "Exec 1"),
    destructive.cueRemovalDialog("Opening", 0, 0), destructive.cueRemovalDialog("Opening", 1, 1), destructive.cueRemovalDialog("Opening", 5, 3),
    destructive.timelinePlacementRemovalDialog(5, "1 · Opening", 0), destructive.timelinePlacementRemovalDialog(5, "1 · Opening", 4000),
  ];
  const invariant = new Set(["OK"]);
  for (const dialog of samples) {
    const strings = [
      dialog.title,
      ...(typeof dialog.message === "string" ? [dialog.message] : dialog.message),
      dialog.confirmLabel ?? "OK",
      dialog.cancelLabel ?? "Cancel",
      service.confirmDialog.length >= 0 ? (dialog.tone === "danger" ? "CAUTION" : "CONFIRM") : "",
    ];
    for (const text of strings) {
      if (invariant.has(text) || /^[A-Za-z]:\\/.test(text)) continue; // a file path is data
      const japanese = localization.translateUiText(text, "ja");
      assert.notEqual(japanese, text, `missing Japanese for dialog text: ${text}`);
      assert.match(japanese, /[\u3040-\u30ff\u4e00-\u9fff]/, `Japanese text expected for: ${text}`);
    }
    assert.ok(dialog.title.endsWith("?"), `a question: ${dialog.title}`);
    assert.ok(dialog.message && (typeof dialog.message === "string" ? dialog.message.length : dialog.message.length) > 0, `names its consequence: ${dialog.title}`);
    if (/^Remove|^Delete|^Discard|^Blackout|^Recover|^Create a new project|^Disarm|^Rotate|^Revoke|^Abandon|^Adopt/.test(dialog.title)) {
      assert.equal(dialog.tone, "danger", `a destructive question starts on Cancel: ${dialog.title}`);
    }
  }
  assert.equal(localization.translateUiText(copy.blackoutDialog(true).title, "ja"), "映像をブラックアウトしますか？");
  assert.equal(localization.translateUiText(copy.blackoutDialog(true).message, "ja"), "本番出力が即座に黒になります。");
  assert.equal(localization.translateUiText(copy.removeFixturesDialog(1, "Front Spot 1").title, "ja"), "灯体「Front Spot 1」を削除しますか？");
  assert.equal(localization.translateUiText(copy.installUpdateDialog("1.2.3").title, "ja"), "Syndocal 1.2.3 をインストールしますか？");
  assert.equal(copy.blackoutDialog(true).tone, "danger");
  // Standby explanations are plain in-app text (never a dialog) with Japanese for every reason.
  for (const [english, japanese] of [
    ["Standby: this machine is not driving outputs", "スタンバイ: このマシンは出力を制御していません"],
    ["Fenced: outputs are held while the machine output role changes", "保護中: 出力ロールの切り替え中は出力が保留されています"],
    ["Output off: no active output lease on this machine", "出力オフ: このマシンには有効な出力リースがありません"],
    ["No blackout was applied. Standby: this machine is not driving outputs.", "ブラックアウトは適用されませんでした。スタンバイ: このマシンは出力を制御していません。"],
  ]) assert.equal(localization.translateUiText(english, "ja"), japanese, `Standby wording localized: ${english}`);
  for (const word of ["Standby", "Fenced", "Output off"]) assert.notEqual(localization.translateUiText(word, "ja"), word, `short key label localized: ${word}`);
  assert.equal(copy.blackoutDialog(false).tone, "danger", "Recover is guarded like Blackout");

  console.log(`in-app dialog service and wording ok (${samples.length} dialogs localized)`);
} finally {
  await server.close();
  await rm(cacheDir, { recursive: true, force: true }).catch(() => {});
}
