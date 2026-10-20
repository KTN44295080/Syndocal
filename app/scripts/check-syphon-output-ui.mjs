import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

// Evaluate the actual TSX controls with a small virtual-element host. Native
// backend availability is the authority; this check never opens a device.
const dataUrl = (source) => `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const solidUrl = dataUrl(`
export const createSignal = (initial) => { let value = initial; return [() => value, (next) => { value = next; }]; };
export const createMemo = (compute) => compute;
export const createEffect = () => {};
export const onMount = () => {};
export const Show = (props) => props.when
  ? typeof props.children === "function" ? props.children(() => props.when) : props.children
  : props.fallback;
export const For = (props) => (props.each ?? []).map((value, index) => props.children(value, () => index));
`);
const elementHost = `
const Fragment = (props) => props.children;
const h = (tag, props, ...children) => {
  const values = { ...props, children: children.length === 1 ? children[0] : children };
  return typeof tag === "function" ? tag(values) : { tag, props: values, children };
};
`;
const source = (path) => readFile(new URL(`../src/${path}`, import.meta.url), "utf8");
const moduleUrl = async (path, replacements = {}) => {
  const result = ts.transpileModule(await source(path), {
    fileName: path,
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      jsx: ts.JsxEmit.React,
      jsxFactory: "h",
      jsxFragmentFactory: "Fragment",
    },
    reportDiagnostics: true,
  });
  assert.equal(result.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length, 0);
  let javascript = result.outputText.replaceAll('from "solid-js"', `from "${solidUrl}"`);
  for (const [specifier, url] of Object.entries(replacements)) {
    javascript = javascript.replaceAll(`from "${specifier}"`, `from "${url}"`);
  }
  return dataUrl(elementHost + javascript);
};
const localizationUrl = await moduleUrl("uiLocalization.ts");
const policyUrl = await moduleUrl("liveVideoMonitorPolicy.ts");
const { VideoOutputCreatePanel } = await import(await moduleUrl("components/VideoOutputCreatePanel.tsx", {
  "../uiLocalization": localizationUrl,
}));
const { VideoOutputConfigPanel } = await import(await moduleUrl("components/VideoOutputConfigPanel.tsx"));
const { LiveVideoOutputHealthReadout } = await import(await moduleUrl("components/LiveVideoOutputHealthReadout.tsx", {
  "../liveVideoMonitorPolicy": policyUrl,
}));
const { translateUiText } = await import(localizationUrl);
const nodes = (tree) => Array.isArray(tree) ? tree.flatMap(nodes)
  : tree && typeof tree === "object" ? [tree, ...nodes(tree.children ?? [])] : [];
const text = (tree) => Array.isArray(tree) ? tree.map(text).join("")
  : tree && typeof tree === "object" ? text(tree.children ?? [])
    : tree === undefined || tree === null || typeof tree === "boolean" ? "" : String(tree);
const syphonOptions = (tree) => nodes(tree).filter((node) => node.tag === "option" && node.props.value === "SyphonServer");
const draft = { label: "Local video", kind: "Display", width: 1920, height: 1080, monitor_id: 0, fullscreen: true, endpoint_name: "Syndocal" };
const create = (kind, syphonAvailable) => VideoOutputCreatePanel({
  ...draft, kind, syphonAvailable, fadeMs: 0, monitorId: 0, endpoint: "Syndocal",
});
const config = (kind, syphonAvailable) => VideoOutputConfigPanel({
  output: { ...draft, id: 30, kind }, draft: { ...draft, kind }, syphonAvailable,
});

for (const render of [create, config]) {
  for (const availability of [false, undefined]) {
    assert.equal(syphonOptions(render("Display", availability)).length, 0,
      "unavailable or unknown native availability must not offer a new Syphon output");
    const preserved = syphonOptions(render("SyphonServer", availability));
    assert.equal(preserved.length, 1, "a saved unsupported selection must remain identifiable");
    assert.equal(preserved[0].props.disabled, true);
    assert.match(text(preserved[0]), /macOS only/);
  }
  const available = syphonOptions(render("Display", true));
  assert.equal(available.length, 1, "a native available Syphon backend offers the type");
  assert.notEqual(available[0].props.disabled, true);
}
for (const availability of [false, true]) {
  const button = nodes(create("SyphonServer", availability)).find((node) => node.props["data-video-output-add"]);
  assert.ok(button);
  assert.equal(button.props.disabled, true, "the retired external creation path stays disabled");
  assert.equal(button.props.onClick, undefined, "type visibility must not bypass canonical output authority");
}
const unsupportedConfig = config("SyphonServer", false);
assert.match(text(unsupportedConfig), /saved output is preserved/);
assert.equal(nodes(unsupportedConfig).find((node) => node.tag === "button").props.disabled, true);
assert.match(text(create("SyphonServer", true)), /Creating external outputs is not available yet/);

const health = LiveVideoOutputHealthReadout({ outputs: [{
  kind: "syphon", outputId: 30, measurementBoundary: "sender", sampleCount: 2,
  deliveredFps: 29.9, targetFps: 30, lastFrameAgeMs: 0, droppedFrames: 0,
}] });
assert.match(text(health), /Syphon #30/);
assert.doesNotMatch(text(health), /NDI/);
assert.match(text(health), /29\.9fps/);
assert.match(nodes(health).find((node) => node.props["data-output-health-id"] === "syphon:30").props.title,
  /Receiver delivery is not measured/);

for (const value of [
  "Syphon Server",
  "Syphon Server (macOS only)",
  "Syphon output",
  "Server name",
  "Syphon output is available on macOS only.",
  "Syphon output is available for configured project outputs. Creating external outputs is not available yet.",
  "Publishes video to Syphon clients on this Mac. Enable Output to start; Standby stops transmission.",
  "Syphon output is available on macOS only. This saved output is preserved; choose another type to use it on this computer.",
  "Syphon Metal output is built in; servers initialize on demand. Syphon input is not implemented.",
  "Syphon input is not implemented",
]) {
  assert.equal(translateUiText(value, "en"), value);
  assert.match(translateUiText(value, "ja"), /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u, value);
}
console.log("Syphon output UI: PASS (native availability, saved selection, creation fence, health, JA/EN)");
