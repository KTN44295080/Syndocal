import { For } from "solid-js";

interface TouchColorPalettePanelProps {
  colors: string[];
  onSetColor: (color: string) => void | Promise<void>;
}

export function TouchColorPalettePanel(props: TouchColorPalettePanelProps) {
  return (
    <div class="touchColorGrid">
      <For each={props.colors}>
        {(color) => (
          <button
            class="colorSwatch"
            style={{ "background-color": color }}
            title={color}
            onClick={() => void props.onSetColor(color)}
            aria-label={`Set color ${color}`}
          />
        )}
      </For>
    </div>
  );
}
