import type { AppStatus } from "../statusModel";

interface AppStatusLineProps {
  status: AppStatus;
}

export function AppStatusLine(props: AppStatusLineProps) {
  return (
    <footer
      class={`appStatusLine ${props.status.tone}`}
      data-status-tone={props.status.tone}
      role="status"
      aria-live="polite"
      aria-atomic="true"
      title={props.status.text}
    >
      <span class="appStatusIndicator" aria-hidden="true" />
      <span class="appStatusText">{props.status.text}</span>
    </footer>
  );
}
