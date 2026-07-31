import { Show } from "solid-js";

interface ProfileLoadPanelProps {
  title: string;
  foldSources?: boolean;
  gdtfPath: string;
  gdtfShareUrl: string;
  onGdtfPath: (value: string) => void;
  onGdtfShareUrl: (value: string) => void;
  onBrowse: () => void | Promise<void>;
  onLoadGdtf: () => void | Promise<void>;
  onDownloadGdtf: () => void | Promise<void>;
}

export function ProfileLoadPanel(props: ProfileLoadPanelProps) {
  const sourceRows = () => (
    <div class="profileImportRows">
      <div class="profileSourceBlock" data-profile-import-local>
        <strong>Local Profile</strong>
        <label>
          File
          <input
            value={props.gdtfPath}
            onInput={(event) => props.onGdtfPath(event.currentTarget.value)}
            placeholder="C:\\path\\fixture.gdtf"
          />
        </label>
        <div class="buttonRow">
          <button onClick={props.onBrowse}>Browse</button>
          <button class="primary" onClick={props.onLoadGdtf}>Load</button>
        </div>
      </div>
      <div class="profileSourceBlock" data-profile-import-share>
        <strong>GDTF Share</strong>
        <label>
          Profile URL
          <input
            value={props.gdtfShareUrl}
            onInput={(event) => props.onGdtfShareUrl(event.currentTarget.value)}
            placeholder="https://gdtf-share.com/.../fixture.gdtf"
          />
        </label>
        <button onClick={props.onDownloadGdtf}>Download</button>
      </div>
    </div>
  );

  return (
    <section class="profileLoadPanel">
      <header class="profileLoadHeader">
        <h2>{props.title}</h2>
        <span>GDTF</span>
      </header>
      <Show
        when={props.foldSources}
        fallback={sourceRows()}
      >
        <details class="profileImportDisclosure" data-profile-import-disclosure>
          <summary>GDTF import</summary>
          {sourceRows()}
        </details>
      </Show>
    </section>
  );
}
