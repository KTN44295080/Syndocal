import { Show, type JSX } from "solid-js";

/** The Broadcast pane header shared by every workspace pane: a label-block
 * title, a context readout, optional tabs/tools and 44px icon actions. The
 * visual rules live in `WorkspacePaneChrome.css` and apply the same way to the
 * legacy `.panelHeader` and `.controlContextHeader` headers. */
export interface WorkspacePaneHeaderProps {
  title: JSX.Element;
  titleId?: string;
  context?: JSX.Element;
  tabs?: JSX.Element;
  tools?: JSX.Element;
  actions?: JSX.Element;
}

export function WorkspacePaneHeader(props: WorkspacePaneHeaderProps) {
  return (
    <header class="workspacePaneHeader" data-workspace-pane-header>
      <h2 class="workspacePaneTitle" id={props.titleId}>{props.title}</h2>
      <Show when={props.context}>
        <span class="workspacePaneContext">{props.context}</span>
      </Show>
      {props.tabs}
      <div class="workspacePaneTools">{props.tools}</div>
      <Show when={props.actions}>
        <div class="workspacePaneActions" role="group" aria-label="Pane controls">{props.actions}</div>
      </Show>
    </header>
  );
}

export interface PaneIconButtonProps {
  kind: "collapse" | "maximise";
  /** Collapsed (collapse) or maximised (maximise). */
  active: boolean;
  controls?: string;
  onToggle: () => void;
  data?: Record<string, string>;
}

const labels = {
  collapse: { off: "Collapse pane", on: "Expand pane" },
  maximise: { off: "Maximise pane", on: "Restore pane" },
} as const;

/** One 44px chrome icon button. The label is a stable action name; the state is
 * carried by `aria-pressed` and the icon, not by colour alone. */
export function PaneIconButton(props: PaneIconButtonProps) {
  const label = () => labels[props.kind][props.active ? "on" : "off"];
  return (
    <button
      type="button"
      class="workspacePaneButton"
      classList={{ active: props.active }}
      aria-pressed={props.active}
      aria-controls={props.controls}
      aria-label={label()}
      title={label()}
      data-pane-action={props.kind}
      {...(props.data ?? {})}
      onClick={() => props.onToggle()}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <Show
          when={props.kind === "collapse"}
          fallback={
            <Show
              when={props.active}
              fallback={<path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7" />}
            >
              <path d="M20 10h-6V4M4 14h6v6M14 10l7-7M3 21l7-7" />
            </Show>
          }
        >
          <Show when={props.active} fallback={<path d="M6 15l6-6 6 6" />}>
            <path d="M6 9l6 6 6-6" />
          </Show>
        </Show>
      </svg>
    </button>
  );
}
