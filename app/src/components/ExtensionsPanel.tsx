import { Show } from "solid-js";

export interface ExtensionStatus { configured: boolean; dj_link_enabled: boolean; installer_managed?: boolean; error: string | null; }
export function ExtensionsPanel(props: { status: ExtensionStatus; busy: boolean; onChange: (enabled: boolean) => void | Promise<void> }) {
  return <section class="ioOperatorSurface" data-extensions-panel>
    <header class="ioDeskHeader"><div><h2>Extensions</h2><p>Choose the integrations you use. DMX, video and MIDI are included in the core app.</p></div></header>
    <section class="ioConnectionDesk">
      <h3>DJ Link</h3>
      <p>Track triggers and timeline synchronization with your DJ computer.</p>
      <p data-extension-installed-state>{props.status.dj_link_enabled ? "Installed" : "Not installed"}</p>
      <p>Choose or change extensions in the Syndocal installer.</p>
      <Show when={props.status.error}><p role="alert" data-no-localize>{props.status.error}</p></Show>
    </section>
  </section>;
}
