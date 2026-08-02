import type { AppStatus } from "../statusModel";

interface AppStatusLineProps {
  status: AppStatus;
  actionLabel?: string | null;
  onAction?: () => void;
}

export function AppStatusLine(props: AppStatusLineProps) {
  return (
    <footer
      class={`appStatusLine ${props.status.tone}`}
      data-status-tone={props.status.tone}
      data-status-key={props.status.key}
      role="status"
      aria-live="polite"
      aria-atomic="true"
      title={props.status.text}
    >
      <span class="appStatusIndicator" aria-hidden="true" />
      <span class="appStatusText">{props.status.text}</span>
      {props.actionLabel && props.onAction ? (
        <button type="button" class="appStatusAction" onClick={props.onAction}>
          {props.actionLabel}
        </button>
      ) : null}
    </footer>
  );
}
