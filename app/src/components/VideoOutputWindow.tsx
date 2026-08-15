import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createEffect, createSignal, onCleanup, Show, untrack } from "solid-js";
import type { VideoFrame } from "../types";
import type { FrontendTauriInvokeCommand } from "../tauriInvokeCommands";
import { drawVideoFrameToCanvas } from "../videoFrameCanvas";

const isTauriRuntime = () =>
  typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const invoke = <T,>(command: FrontendTauriInvokeCommand, args?: Record<string, unknown>) => {
  if (!isTauriRuntime()) {
    return Promise.reject(new Error("Syndocal desktop backend is not connected in this output window."));
  }
  return tauriInvoke<T>(command, args);
};

export const readVideoOutputWindowId = () => {
  const raw = new URLSearchParams(window.location.search).get("videoOutputId");
  if (!raw) {
    return null;
  }
  const outputId = Number(raw);
  return Number.isInteger(outputId) && outputId > 0 ? outputId : null;
};

export const readVideoOutputTestPattern = () => new URLSearchParams(window.location.search).get("testPattern") === "1";

const videoOutputWindowRenderSize = () => {
  const dpr = Math.min(Math.max(window.devicePixelRatio || 1, 1), 2);
  let width = Math.max(1, Math.floor(window.innerWidth * dpr));
  let height = Math.max(1, Math.floor(window.innerHeight * dpr));
  const maxPixels = 1280 * 720;
  if (width * height > maxPixels) {
    const scale = Math.sqrt(maxPixels / (width * height));
    width = Math.max(1, Math.floor(width * scale));
    height = Math.max(1, Math.floor(height * scale));
  }
  return { width, height };
};

const videoOutputWindowTargetFps = 60;
const videoOutputWindowTargetFrameMs = 1000 / videoOutputWindowTargetFps;

export function VideoOutputWindow(props: { outputId: number; testPattern: boolean }) {
  let frameCanvas: HTMLCanvasElement | undefined;
  const [hasFrame, setHasFrame] = createSignal(false);
  const [status, setStatus] = createSignal("Waiting for output frame");
  const [frameCount, setFrameCount] = createSignal(0);
  const [renderMs, setRenderMs] = createSignal(0);
  const [fps, setFps] = createSignal(0);
  const [droppedFrames, setDroppedFrames] = createSignal(0);
  const [windowHealth, setWindowHealth] = createSignal<"ok" | "warn" | "bad">("warn");
  const [overlayVisible, setOverlayVisible] = createSignal(true);
  const [overlayPinned, setOverlayPinned] = createSignal(false);

  createEffect(() => {
    document.body.classList.add("outputBody");
    let cancelled = false;
    let animationFrameId = 0;
    let timeoutId = 0;
    let overlayTimeoutId = 0;
    let inFlight = false;
    let lastFrameAt = performance.now();
    let lastAttemptAt = 0;
    let consecutiveErrors = 0;

    const scheduleOverlayHide = () => {
      window.clearTimeout(overlayTimeoutId);
      if (!untrack(overlayPinned)) {
        overlayTimeoutId = window.setTimeout(() => setOverlayVisible(false), 2500);
      }
    };

    const revealOverlay = () => {
      setOverlayVisible(true);
      scheduleOverlayHide();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() === "i") {
        const nextPinned = !overlayPinned();
        setOverlayPinned(nextPinned);
        setOverlayVisible(true);
        if (!nextPinned) {
          scheduleOverlayHide();
        }
      } else if (event.key.toLowerCase() === "h" || event.key === "Escape") {
        setOverlayPinned(false);
        window.clearTimeout(overlayTimeoutId);
        setOverlayVisible(false);
      }
    };

    window.addEventListener("mousemove", revealOverlay);
    window.addEventListener("keydown", handleKeyDown);
    scheduleOverlayHide();

    const scheduleNext = (delayMs = 0) => {
      if (cancelled) {
        return;
      }
      if (delayMs > 0) {
        timeoutId = window.setTimeout(() => {
          animationFrameId = window.requestAnimationFrame(renderFrame);
        }, delayMs);
      } else {
        animationFrameId = window.requestAnimationFrame(renderFrame);
      }
    };

    const renderFrame = async (timestamp: number) => {
      if (cancelled) {
        return;
      }
      if (inFlight) {
        scheduleNext();
        return;
      }
      const sinceLastAttempt = timestamp - lastAttemptAt;
      if (lastAttemptAt > 0 && sinceLastAttempt < videoOutputWindowTargetFrameMs) {
        scheduleNext(videoOutputWindowTargetFrameMs - sinceLastAttempt);
        return;
      }
      lastAttemptAt = timestamp;
      inFlight = true;
      const { width, height } = videoOutputWindowRenderSize();
      const start = performance.now();
      try {
        const frame = await invoke<VideoFrame>(
          props.testPattern ? "get_debug_video_output_test_pattern" : "get_debug_video_output_preview",
          props.testPattern
            ? {
                outputId: props.outputId,
                width,
                height,
              }
            : {
                outputId: props.outputId,
                width,
                height,
                decodeBudget: 1,
              },
        );
        if (!cancelled && frameCanvas) {
          drawVideoFrameToCanvas(frameCanvas, frame);
          const now = performance.now();
          const interval = Math.max(1, now - lastFrameAt);
          lastFrameAt = now;
          const missedFrames = Math.max(0, Math.round(interval / videoOutputWindowTargetFrameMs) - 1);
          if (missedFrames > 0) {
            setDroppedFrames((count) => count + missedFrames);
          }
          consecutiveErrors = 0;
          setHasFrame(true);
          setFrameCount((count) => count + 1);
          setRenderMs(now - start);
          setFps(1000 / interval);
          setWindowHealth(now - start > 50 || missedFrames > 2 ? "bad" : now - start > videoOutputWindowTargetFrameMs || missedFrames > 0 ? "warn" : "ok");
          setStatus(`${props.testPattern ? "Test pattern" : "Output"} ${props.outputId} / ${frame.width}x${frame.height} / pts ${frame.pts_ms}ms`);
        }
      } catch (error) {
        if (!cancelled) {
          consecutiveErrors += 1;
          setStatus(String(error));
          setWindowHealth("bad");
        }
      } finally {
        inFlight = false;
        if (!cancelled) {
          scheduleNext(consecutiveErrors >= 3 ? 500 : 0);
        }
      }
    };

    scheduleNext();
    onCleanup(() => {
      cancelled = true;
      window.cancelAnimationFrame(animationFrameId);
      window.clearTimeout(timeoutId);
      window.clearTimeout(overlayTimeoutId);
      window.removeEventListener("mousemove", revealOverlay);
      window.removeEventListener("keydown", handleKeyDown);
      document.body.classList.remove("outputBody");
    });
  });

  return (
    <main class="videoOutputWindow">
      <canvas ref={frameCanvas} class="videoOutputFrame" aria-label={`Video output ${props.outputId}`} />
      <Show when={!hasFrame()}>
        <div class="videoOutputStatus">{status()}</div>
      </Show>
      <div class={`videoOutputOverlay ${windowHealth()} ${overlayVisible() ? "" : "hidden"}`}>
        <strong>{props.testPattern ? "Pattern" : "Live"} {props.outputId}</strong>
        <span>{fps().toFixed(1)} fps</span>
        <span>{renderMs().toFixed(1)} ms</span>
        <span>{videoOutputWindowTargetFps} Hz target</span>
        <span>#{frameCount()}</span>
        <span>drop {droppedFrames()}</span>
        <small>{status()}</small>
      </div>
    </main>
  );
}
