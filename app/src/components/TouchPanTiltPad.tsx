import type { JSX } from "solid-js";

interface TouchPanTiltPadProps {
  panValue: number;
  tiltValue: number;
  limitOverlayStyle: JSX.CSSProperties;
  onPointerValue: (event: PointerEvent) => void | Promise<void>;
}

export function TouchPanTiltPad(props: TouchPanTiltPadProps) {
  const setPointerValue = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
    void props.onPointerValue(event);
  };

  return (
    <div
      class="panTiltPad touchPanTiltPad"
      role="slider"
      aria-label="Touch pan tilt pad"
      aria-valuetext={`Pan ${props.panValue}, Tilt ${props.tiltValue}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        setPointerValue(event);
      }}
      onPointerMove={(event) => {
        if (event.buttons === 1) {
          setPointerValue(event);
        }
      }}
      onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
    >
      <b class="panTiltLimitWindow" style={props.limitOverlayStyle} />
      <i
        style={{
          left: `${(props.panValue / 65535) * 100}%`,
          top: `${100 - (props.tiltValue / 65535) * 100}%`,
        }}
      />
    </div>
  );
}
