/** The empty Perform matrix starts the first show through the existing
 * first-run flow (`createFirstRunVjShow`). Native drops are routed by this
 * attribute; a page-level drop reads the paths it can, else opens the picker. */
export const videoPerformFirstShowDropSelector = "[data-video-perform-first-show-drop]";

export const isVideoPerformFirstShowDropTarget = (target: Element | null | undefined): boolean =>
  Boolean(target?.closest(videoPerformFirstShowDropSelector));

/** Converts one `file:` URI to a local path. Windows drive and UNC URIs become
 * backslash paths; anything that is not a file URI is rejected. */
export function fileUriToPath(uri: string): string | null {
  let url: URL;
  try { url = new URL(uri.trim()); } catch { return null; }
  if (url.protocol !== "file:") return null;
  let path: string;
  try { path = decodeURIComponent(url.pathname); } catch { return null; }
  if (url.host) return `\\\\${url.host}${path.replaceAll("/", "\\")}`;
  if (/^\/[A-Za-z]:\//.test(path)) return path.slice(1).replaceAll("/", "\\");
  return path || null;
}

/** File paths a page-level drop exposes through `text/uri-list`. WebView2 file
 * drops usually carry only `File` objects without paths; callers then fall
 * back to the file picker so the drop still starts the same flow. */
export function droppedFilePaths(transfer: Pick<DataTransfer, "getData"> | null | undefined): string[] {
  if (!transfer) return [];
  let list = "";
  try { list = transfer.getData("text/uri-list"); } catch { return []; }
  return list.split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map(fileUriToPath)
    .filter((path): path is string => Boolean(path));
}

export const dragCarriesFiles = (transfer: Pick<DataTransfer, "types"> | null | undefined): boolean =>
  Boolean(transfer && Array.from(transfer.types ?? []).includes("Files"));
