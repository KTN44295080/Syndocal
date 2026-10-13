import { Show } from "solid-js";

export interface ExtensionStatus { configured: boolean; dj_link_enabled: boolean; error: string | null; }
export function ExtensionsPanel(props: { status: ExtensionStatus; busy: boolean; onChange: (enabled: boolean) => void | Promise<void> }) {
  return <section class="ioOperatorSurface" data-extensions-panel>
    <header class="ioDeskHeader"><div><h2>Extensions</h2><p>Choose the integrations you use. DMX, video and MIDI are included in the core app.</p></div></header>
    <section class="ioConnectionDesk">
      <h3>DJ Link</h3>
      <p>Track triggers and timeline synchronization with your DJ computer.</p>
      <label class="checkbox"><input type="checkbox" checked={props.status.dj_link_enabled} disabled={props.busy || !!props.status.error} onChange={event => void props.onChange(event.currentTarget.checked)} />Enable DJ Link extension</label>
      <Show when={!props.status.configured && !props.status.error}>
        <button type="button" class="primary" disabled={props.busy} onClick={() => void props.onChange(false)}>Continue with core app</button>
      </Show>
      <Show when={props.status.error}><p role="alert" data-no-localize>{props.status.error}</p></Show>
    </section>
  </section>;
}
