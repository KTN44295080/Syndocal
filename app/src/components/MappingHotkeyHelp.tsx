import { For } from "solid-js";

type MappingShortcut = {
  key: string;
  action: string;
};

type MappingShortcutGroup = {
  label: string;
  shortcuts: MappingShortcut[];
};

const mappingShortcutGroups: MappingShortcutGroup[] = [
  {
    label: "Tools",
    shortcuts: [
      { key: "S", action: "Select" },
      { key: "P", action: "Place" },
      { key: "R", action: "Rotate" },
      { key: "H", action: "Pan view" },
    ],
  },
  {
    label: "View",
    shortcuts: [
      { key: "F / Shift+F", action: "Fit visible / selection" },
      { key: "+ / -", action: "Zoom in / out" },
      { key: "0", action: "Reset view" },
      { key: "L B G V O", action: "Toggle map layers" },
      { key: "Shift+5", action: "Toggle levels" },
    ],
  },
  {
    label: "Selection",
    shortcuts: [
      { key: "Ctrl/Cmd+A", action: "Pick visible" },
      { key: "Ctrl/Cmd+Shift+A", action: "Clear pick" },
      { key: "I / Shift+I", action: "Pick / add inside object" },
      { key: "Arrow keys", action: "Nudge by snap" },
      { key: "Shift+Arrow", action: "Nudge 5x" },
      { key: "Ctrl/Cmd+D", action: "Duplicate pick" },
      { key: "Delete", action: "Remove pick" },
    ],
  },
  {
    label: "Arrange",
    shortcuts: [
      { key: "1 / 2 / 3", action: "Line / grid / circle" },
      { key: "Shift+1 / 2", action: "Line / grid on object" },
      { key: "X / Z", action: "Align axis" },
      { key: "Shift+X / Z", action: "Distribute axis" },
      { key: "[ / ]", action: "Rotate -15 / +15" },
      { key: "Shift+[ / ]", action: "Mirror X / Z" },
      { key: "4", action: "Rotate 180" },
    ],
  },
  {
    label: "Live Flags",
    shortcuts: [
      { key: "Q", action: "Highlight" },
      { key: "W", action: "Solo" },
      { key: "E", action: "Park" },
      { key: "Esc", action: "Close help / cancel tool" },
    ],
  },
];

interface MappingHotkeyHelpProps {
  onClose: () => void;
}

export function MappingHotkeyHelp(props: MappingHotkeyHelpProps) {
  return (
    <aside class="mappingHotkeyHelp" aria-label="2D mapping keyboard shortcuts">
      <div class="mappingHotkeyHelpHeader">
        <div>
          <h3>Mapping Shortcuts</h3>
          <span>Active in Setup / Mapping</span>
        </div>
        <button type="button" onClick={props.onClose} aria-label="Close mapping shortcut help" title="Close (Esc)">
          x
        </button>
      </div>
      <div class="mappingHotkeyHelpGrid">
        <For each={mappingShortcutGroups}>
          {(group) => (
            <section>
              <h4>{group.label}</h4>
              <dl>
                <For each={group.shortcuts}>
                  {(shortcut) => (
                    <div>
                      <dt><kbd>{shortcut.key}</kbd></dt>
                      <dd>{shortcut.action}</dd>
                    </div>
                  )}
                </For>
              </dl>
            </section>
          )}
        </For>
      </div>
    </aside>
  );
}
