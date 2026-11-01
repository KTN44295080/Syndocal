/** Windows keeps HTML5 DnD enabled. Resolve only OS-backed File objects via
 * WebView2's AdditionalObjects API; importing still belongs to the picker path. */
export interface FileDropWebView {
  postMessageWithAdditionalObjects(message: unknown, objects: File[]): void;
  addEventListener(type: "message", listener: (event: MessageEvent) => void): void;
  removeEventListener(type: "message", listener: (event: MessageEvent) => void): void;
}

export const unsupportedFileDropMessage = "Drop a .sdc project, visual media, or Audio clip on its matching surface.";

export function installNativeFileDrop<Context>(options: {
  document: Document;
  webview: FileDropWebView;
  onDrop: (paths: string[], target: Element) => void;
  onError: (message: string) => void;
  captureContext: () => Context;
  contextIsCurrent: (context: Context) => boolean;
}): () => void {
  const { document, webview } = options;
  let nextId = 0;
  // One outstanding gesture. A newer drop invalidates an older reply.
  let pending: { id: number; target: Element; context: Context; timer: ReturnType<typeof setTimeout> } | null = null;
  const clear = () => {
    if (pending) clearTimeout(pending.timer);
    pending = null;
  };
  const dragOver = (event: DragEvent) => {
    if (!event.dataTransfer?.types.includes("Files")) return;
    // Permit the DOM drop to reach the existing router on every surface,
    // including project/clip/audio targets and off-target rejection. Never
    // let an external file navigate the WebView away from the application.
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
  };
  const drop = (event: DragEvent) => {
    const files = Array.from(event.dataTransfer?.files ?? []);
    // Internal profile, scene, FX and Timeline drags have no Files. Let their
    // original handlers receive the event and their custom DataTransfer intact.
    if (files.length === 0 || !(event.target instanceof Element)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    // Notify the existing hover owners too (Perform keeps a Solid signal).
    document.querySelectorAll("[data-file-drag-over]").forEach(element => {
      element.dispatchEvent(new DragEvent("dragleave", { bubbles: true }));
    });
    clear();
    const id = ++nextId;
    const context = options.captureContext();
    pending = { id, target: event.target, context, timer: setTimeout(() => {
      clear();
      if (options.contextIsCurrent(context)) options.onError(unsupportedFileDropMessage);
    }, 10_000) };
    try {
      webview.postMessageWithAdditionalObjects({ type: "syndocal:file-drop", id }, files);
    } catch {
      clear();
      options.onError(unsupportedFileDropMessage);
    }
  };
  const message = (event: MessageEvent) => {
    const reply = event.data;
    if (reply?.type !== "syndocal:file-drop-result" || !pending || reply.id !== pending.id) return;
    const { target, context } = pending;
    clear();
    if (!target.isConnected || !options.contextIsCurrent(context)) return;
    if (reply.error || !Array.isArray(reply.paths) || reply.paths.length === 0
      || !reply.paths.every((path: unknown) => typeof path === "string" && path.length > 0)) {
      options.onError(unsupportedFileDropMessage);
      return;
    }
    options.onDrop(reply.paths, target);
  };
  document.addEventListener("drop", drop, true);
  document.addEventListener("dragover", dragOver, true);
  webview.addEventListener("message", message);
  return () => {
    clear();
    document.removeEventListener("drop", drop, true);
    document.removeEventListener("dragover", dragOver, true);
    webview.removeEventListener("message", message);
  };
}
