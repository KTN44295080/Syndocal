interface ProfileLoadPanelProps {
  gdtfPath: string;
  gdtfShareUrl: string;
  onGdtfPath: (value: string) => void;
  onGdtfShareUrl: (value: string) => void;
  onBrowse: () => void | Promise<void>;
  onLoadGdtf: () => void | Promise<void>;
  onDownloadGdtf: () => void | Promise<void>;
}

export function ProfileLoadPanel(props: ProfileLoadPanelProps) {
  return (
    <>
      <h2>Patch</h2>
      <label>
        GDTF path
        <input
          value={props.gdtfPath}
          onInput={(event) => props.onGdtfPath(event.currentTarget.value)}
          placeholder="C:\\path\\fixture.gdtf"
        />
      </label>
      <div class="buttonRow">
        <button onClick={props.onBrowse}>Browse</button>
        <button class="primary" onClick={props.onLoadGdtf}>Load GDTF</button>
      </div>
      <label>
        GDTF Share URL
        <input
          value={props.gdtfShareUrl}
          onInput={(event) => props.onGdtfShareUrl(event.currentTarget.value)}
          placeholder="https://gdtf-share.com/.../fixture.gdtf"
        />
      </label>
      <button onClick={props.onDownloadGdtf}>Download GDTF URL</button>
    </>
  );
}
