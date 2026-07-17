export type MixerDrawerId = "audio-in" | "auto-vj" | "reactive";

const storageKey = (id: MixerDrawerId) => `syndocal.mixerDrawer.${id}.v1`;

export const loadMixerDrawerOpen = (id: MixerDrawerId): boolean => {
  try {
    return window.localStorage.getItem(storageKey(id)) === "1";
  } catch {
    return false;
  }
};

export const saveMixerDrawerOpen = (id: MixerDrawerId, open: boolean) => {
  try {
    window.localStorage.setItem(storageKey(id), open ? "1" : "0");
  } catch {
    // Quota/unavailable failures only lose persistence; the toggle still works this session.
  }
};

interface MixerDrawerBarProps {
  id: MixerDrawerId;
  title: string;
  status: string;
  open: boolean;
  onToggle: () => void;
}

export function MixerDrawerBar(props: MixerDrawerBarProps) {
  return (
    <button
      type="button"
      class={`mixerDrawerBar ${props.open ? "open" : ""}`}
      data-mixer-drawer-toggle={props.id}
      aria-expanded={props.open}
      onClick={() => props.onToggle()}
    >
      <span class="mixerDrawerChevron" aria-hidden="true">{props.open ? "▾" : "▸"}</span>
      <strong>{props.title}</strong>
      <span class="mixerDrawerStatus" data-no-localize>{props.status}</span>
    </button>
  );
}
