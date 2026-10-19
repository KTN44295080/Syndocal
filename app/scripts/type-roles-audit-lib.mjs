// Type-role audit shared by check-theme-browser.mjs and the Task 14 harness.
// `typeRoleAudit` runs INSIDE the page (pass it to page.evaluate): it walks every
// visible text node and reports violations of the readability rules defined by the
// type-role tokens in theme.css:
//   - no visible text under 13px (effective size, so scaled SVG text counts too);
//   - interactive labels and list items are at least 14px (condensed chrome at least 13px);
//   - the condensed chrome face is used only where the stylesheet declared --type-role: chrome;
//   - pane titles are calm (transparent fill, primary text colour, 3px accent bar), never an inverse block.
export function typeRoleAudit() {
  const MIN_TEXT = 13, MIN_CONTROL = 14, MIN_CHROME = 13;
  // Interactive labels and list items. Field labels (<label>) and secondary text (counts, key caps,
  // <small> sub-lines) are plain text: 13px is enough for them.
  const interactiveSelector = "button, a[href], summary, select, [role=button], [role=tab], [role=menuitem], [role=option], [role=treeitem], [role=listitem], li";
  const secondarySelector = "small, kbd, label, [class*=Count], [class*=count], [class*=Meta], [class*=meta], [class*=Hint], [class*=hint], [class*=Status], [class*=State], [class*=Chip]";
  const titleSelector = ".workspacePaneTitle, .workspacePaneHeader > h2, .controlContextHeader > h2, .panelHeader h2, .panelHeader h3, .profileLoadHeader h2, .profileLoadHeader h3, .mappingPersistentWorkspaceGrid > .mappingGroupStrip > span";
  const describe = (el) => {
    const parts = [];
    for (let node = el, depth = 0; node && node.nodeType === 1 && depth < 3; node = node.parentElement, depth++) {
      const cls = typeof node.className === "string" ? node.className.trim().split(/\s+/)[0] : "";
      parts.push(node.tagName.toLowerCase() + (cls ? "." + cls : ""));
    }
    return parts.join(" < ");
  };
  const visible = (el, rect) => {
    if (!rect.width || !rect.height || rect.bottom < 0 || rect.top >= innerHeight || rect.right < 0 || rect.left >= innerWidth) return false;
    const cs = getComputedStyle(el);
    if (cs.visibility !== "visible" || cs.display === "none" || Number(cs.opacity) === 0) return false;
    // Clipped by a scroll/overflow ancestor: the text is not actually shown.
    for (let node = el.parentElement; node && node !== document.body; node = node.parentElement) {
      const o = getComputedStyle(node);
      if (/(hidden|clip|auto|scroll)/.test(o.overflowX + o.overflowY)) {
        const r = node.getBoundingClientRect();
        if (rect.bottom <= r.top || rect.top >= r.bottom || rect.right <= r.left || rect.left >= r.right) return false;
      }
    }
    return true;
  };
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const under = [], controlsUnder = [], condensedOutside = [], upperOutside = [], boldOutside = [];
  let node, measured = 0, min = Infinity;
  const seen = new Set();
  while ((node = walker.nextNode())) {
    const text = node.textContent.trim(), el = node.parentElement;
    if (!text || !/[A-Za-z0-9぀-ヿ一-鿿]/.test(text) || !el || el.closest("script, style, [data-type-audit-skip]")) continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    const rect = range.getBoundingClientRect();
    if (!visible(el, rect)) continue;
    const cs = getComputedStyle(el);
    let size = parseFloat(cs.fontSize);
    // SVG text is sized in user units; its on-screen size is the unit size times the CTM scale.
    if (el instanceof SVGElement && typeof el.getScreenCTM === "function") {
      const ctm = el.getScreenCTM();
      if (ctm) size *= Math.hypot(ctm.a, ctm.b);
    }
    // font-size:0 hides the whole word behind a ::first-letter initial (tiny patch blocks); the initial is what shows.
    if (size === 0) continue;
    measured++;
    min = Math.min(min, size);
    const role = cs.getPropertyValue("--type-role").trim();
    const first = cs.fontFamily.split(",")[0].replace(/["']/g, "").trim();
    const condensed = /^(Bahnschrift|Arial Narrow)/.test(first);
    const chrome = role === "chrome";
    const key = describe(el) + "|" + text.slice(0, 40);
    const entry = { text: text.slice(0, 48), where: describe(el), size: Math.round(size * 100) / 100, family: first, weight: cs.fontWeight };
    if (size + 0.01 < MIN_TEXT) under.push(entry);
    else if (!seen.has(key)) {
      // SVG labels (timeline blocks, stage) follow the 13px floor; they are drawn, not laid out as controls.
      const interactive = el instanceof SVGElement ? null : el.closest(interactiveSelector);
      const secondary = el.closest(secondarySelector) && el.closest(secondarySelector) !== interactive;
      if (interactive && !secondary && size + 0.01 < (chrome ? MIN_CHROME : MIN_CONTROL)) controlsUnder.push(entry);
    }
    seen.add(key);
    if (condensed && !chrome) condensedOutside.push(entry);
    // Informational: bold (600+) text that is not chrome, a title, or an active/selected state.
    if (!chrome && Number(cs.fontWeight) >= 600 && !el.closest("h1, h2, h3, h4, h5, h6, dialog, [role=dialog], [role=alertdialog], .active, .selected, .live, .armed, .engaged, .latched, .error, .warning, [aria-pressed=true], [aria-selected=true], [aria-checked=true], [aria-current], [class*=Title], [class*=Badge], [class*=State], [class*=Status], .pill, .primary, .danger, button:disabled")) boldOutside.push(entry);
    // Informational: all-caps source text outside chrome (content should be written in normal case).
    if (!chrome && /^[A-Z][A-Z0-9 /+._-]{3,}$/.test(text) && /[A-Z]{4}/.test(text) && !el.closest("[data-no-localize]")) upperOutside.push(entry);
  }
  // Pane titles: calm treatment (3px accent bar + primary text), never an inverse block.
  const accentProbe = document.createElement("i");
  accentProbe.style.color = "var(--theme-accent)";
  document.body.append(accentProbe);
  const accent = getComputedStyle(accentProbe).color;
  accentProbe.style.color = "var(--theme-text)";
  const textColor = getComputedStyle(accentProbe).color;
  accentProbe.remove();
  const titles = [...document.querySelectorAll(titleSelector)].filter((el) => visible(el, el.getBoundingClientRect())).map((el) => {
    const cs = getComputedStyle(el);
    const transparent = /rgba\(\s*0,\s*0,\s*0,\s*0\s*\)|transparent/.test(cs.backgroundColor);
    return {
      text: el.textContent.trim().slice(0, 40), where: describe(el), background: cs.backgroundColor, color: cs.color,
      transparentFill: transparent, primaryText: cs.color === textColor, accentFill: cs.backgroundColor === accent,
      bar: /inset/.test(cs.boxShadow) && /\b3px\b/.test(cs.boxShadow),
      transform: cs.textTransform, size: parseFloat(cs.fontSize),
    };
  });
  return { measured, minPx: min === Infinity ? null : Math.round(min * 100) / 100, under, controlsUnder, condensedOutside, upperOutside, boldOutside, titles };
}
