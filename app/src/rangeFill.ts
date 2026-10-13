/**
 * Keeps `--range-fill` (0%-100%) on every native range input so the shared
 * slider skin can paint the filled part of the track. Pointer and keyboard
 * edits arrive as `input` events; framework updates assign `value`, so the
 * HTMLInputElement value setter is wrapped once; added inputs and min/max
 * changes are picked up by one MutationObserver.
 */

const rangeFillInstalled = Symbol.for("syndocal.rangeFillInstalled");

const finiteOr = (value: string, fallback: number) => {
  const parsed = Number(value);
  return value !== "" && Number.isFinite(parsed) ? parsed : fallback;
};

export const rangeFillPercent = (min: string, max: string, value: string) => {
  const low = finiteOr(min, 0);
  const high = finiteOr(max, 100);
  const current = finiteOr(value, low + (high - low) / 2);
  if (!(high > low)) return 0;
  return Math.min(100, Math.max(0, ((current - low) / (high - low)) * 100));
};

const isRange = (node: unknown): node is HTMLInputElement =>
  node instanceof HTMLInputElement && node.type === "range";

export const syncRangeFill = (input: HTMLInputElement) => {
  const percent = rangeFillPercent(input.min, input.max, input.value);
  const next = `${Math.round(percent * 100) / 100}%`;
  if (input.style.getPropertyValue("--range-fill") !== next) {
    input.style.setProperty("--range-fill", next);
  }
};

const syncTree = (node: Node) => {
  if (isRange(node)) {
    syncRangeFill(node);
    return;
  }
  if (node instanceof Element) {
    for (const input of node.getElementsByTagName("input")) {
      if (input.type === "range") syncRangeFill(input);
    }
  }
};

export function installRangeFillSync(doc: Document = document) {
  const view = doc.defaultView as (Window & typeof globalThis) | null;
  if (!view) return;
  const prototype = view.HTMLInputElement.prototype as HTMLInputElement & {
    [rangeFillInstalled]?: true;
  };
  if (!prototype[rangeFillInstalled]) {
    const descriptor = Object.getOwnPropertyDescriptor(view.HTMLInputElement.prototype, "value");
    if (descriptor?.get && descriptor.set) {
      const { get, set } = descriptor;
      Object.defineProperty(view.HTMLInputElement.prototype, "value", {
        configurable: true,
        enumerable: descriptor.enumerable,
        get,
        set(this: HTMLInputElement, value: string) {
          set.call(this, value);
          if (this.type === "range") syncRangeFill(this);
        },
      });
    }
    Object.defineProperty(prototype, rangeFillInstalled, { value: true });
  }
  doc.addEventListener("input", (event) => {
    if (isRange(event.target)) syncRangeFill(event.target);
  }, true);
  new view.MutationObserver((records) => {
    for (const record of records) {
      if (record.type === "attributes") {
        if (isRange(record.target)) syncRangeFill(record.target);
        continue;
      }
      record.addedNodes.forEach(syncTree);
    }
  }).observe(doc.documentElement, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["min", "max", "value", "type"],
  });
  syncTree(doc.documentElement);
}
