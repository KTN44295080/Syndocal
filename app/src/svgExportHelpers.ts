// SVG/stage-map export helpers extracted from App.tsx.
// DOM/string utilities for inlining computed styles and downloading exported files.
// No SolidJS/state deps.

export const svgExportComputedStyleProperties = [
  "color",
  "display",
  "fill",
  "fill-opacity",
  "filter",
  "font-family",
  "font-size",
  "font-style",
  "font-weight",
  "letter-spacing",
  "line-height",
  "mix-blend-mode",
  "opacity",
  "paint-order",
  "stroke",
  "stroke-dasharray",
  "stroke-linecap",
  "stroke-linejoin",
  "stroke-miterlimit",
  "stroke-opacity",
  "stroke-width",
  "text-anchor",
  "visibility",
  "white-space",
] as const;

export const standaloneSvgExportSelectorsToRemove = [
  ".stageCursorGuide",
  ".mappingMarquee",
  ".stagePlacePreview",
  ".stageObjectHandleLine",
  ".stageObjectRotateHandle",
  ".stageObjectResizeHandle",
  ".stageVideoSurfaceHandleLine",
  ".stageVideoSurfaceRotateHandle",
  ".stageVideoSurfaceScaleHandle",
  ".stageVideoSurfaceCornerHandle",
  ".stageYawHandle",
].join(",");

export const inlineComputedSvgStyles = (source: Element, target: Element) => {
  const computed = window.getComputedStyle(source);
  const style = svgExportComputedStyleProperties
    .map((property) => {
      const value = computed.getPropertyValue(property);
      return value ? `${property}:${value}` : "";
    })
    .filter(Boolean)
    .join(";");
  if (style) {
    target.setAttribute("style", style);
  }

  const sourceChildren = Array.from(source.children);
  const targetChildren = Array.from(target.children);
  sourceChildren.forEach((sourceChild, index) => {
    const targetChild = targetChildren[index];
    if (targetChild) {
      inlineComputedSvgStyles(sourceChild, targetChild);
    }
  });
};

export const downloadTextFile = (fileName: string, text: string, mimeType: string) => {
  const blob = new Blob([text], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  link.style.display = "none";
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 10_000);
};

export const safeExportFileNamePart = (value: string) => {
  const safe = value
    .trim()
    .replace(/[<>:"/\\|?*\x00-\x1f]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return safe || "stage-map";
};
