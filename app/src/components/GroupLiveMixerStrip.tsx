import { GROUP_STROBE_MAX_HZ } from "../groupStrobe";

interface GroupLiveMixerStripProps {
  groupId: string | null;
  fixtureCount: number;
  strobeFixtureCount: number;
  submasterLevel: number;
  strobeHz: number;
  soloed: boolean;
  onSetSubmaster: (groupId: string, level: number) => void | Promise<void>;
  onSetStrobe: (groupId: string, rateHz: number) => void | Promise<void>;
  onSetSolo: (groupId: string, enabled: boolean) => void | Promise<void>;
}

export function GroupLiveMixerStrip(props: GroupLiveMixerStripProps) {
  const disabled = () => !props.groupId || props.fixtureCount === 0;
  const strobeDisabled = () => disabled() || props.strobeFixtureCount === 0;
  return (
    <section class="groupLiveMixerStrip" data-live-mixer-group={props.groupId ?? ""}>
      <div class="groupLiveMixerIdentity">
        <span>Live Mixer</span>
        <strong data-no-localize>{props.groupId ?? "Select group"}</strong>
        <small>{props.fixtureCount} fixture(s)</small>
      </div>
      <label class="groupLiveMixerControl">
        Dimmer
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={props.submasterLevel}
          disabled={disabled()}
          onChange={(event) => props.groupId
            && void props.onSetSubmaster(props.groupId, Number(event.currentTarget.value))}
        />
        <span>{Math.round(props.submasterLevel * 100)}%</span>
      </label>
      <label class="groupLiveMixerControl groupLiveMixerStrobe">
        Strobe
        <input
          type="range"
          min="0"
          max={GROUP_STROBE_MAX_HZ}
          step="1"
          value={props.strobeHz}
          disabled={strobeDisabled()}
          aria-label="Group strobe rate"
          title={props.strobeFixtureCount === 0
            ? "No fixture has GDTF strobe frequency metadata"
            : `${props.strobeFixtureCount} compatible fixture(s)`}
          onChange={(event) => props.groupId
            && void props.onSetStrobe(props.groupId, Number(event.currentTarget.value))}
        />
        <span>{props.strobeHz > 0 ? `${props.strobeHz.toFixed(0)} Hz` : "Off"}</span>
      </label>
      <span class="groupLiveMixerCoverage" data-strobe-compatible-count={props.strobeFixtureCount}>
        {props.strobeFixtureCount}/{props.fixtureCount} GDTF
      </span>
      <button
        type="button"
        class={props.soloed ? "active" : ""}
        disabled={disabled()}
        aria-pressed={props.soloed}
        onClick={() => props.groupId && void props.onSetSolo(props.groupId, !props.soloed)}
      >
        {props.soloed ? "Clear Solo" : "Solo"}
      </button>
    </section>
  );
}
