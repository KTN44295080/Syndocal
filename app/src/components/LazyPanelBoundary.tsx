import { ErrorBoundary, Suspense, type JSX } from "solid-js";

// Imports normally complete immediately from the local installation. A missing
// or stalled chunk must still settle into an actionable state.
export async function loadPanel<T>(load: () => Promise<T>, timeoutMs = 15_000): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      load(),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error("Panel loading timed out")), timeoutMs);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

export function LazyPanelBoundary(props: { label: string; children: JSX.Element; fallbackAction?: JSX.Element }) {
  return (
    <ErrorBoundary fallback={
      <div class="empty" role="alert" aria-label={props.label}>
        <p>Panel could not load. Restart Syndocal and try again.</p>
        {props.fallbackAction}
      </div>
    }>
      <Suspense fallback={
        <div class="empty" role="status" aria-label={props.label} aria-busy="true">
          <p>Loading panel…</p>
          {props.fallbackAction}
        </div>
      }>
        {props.children}
      </Suspense>
    </ErrorBoundary>
  );
}
