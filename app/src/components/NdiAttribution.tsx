import "./NdiAttribution.css";

/**
 * NDI SDK licence attribution wherever NDI is selected: the trademark notice
 * and the official URLs. The URLs are selectable text because the app webview
 * does not open external links.
 */
export function NdiAttribution(props: { class?: string; output?: boolean }) {
  return (
    <p
      class={props.class ?? "hint"}
      data-ndi-trademark-notice={props.output ? undefined : ""}
      data-ndi-output-attribution={props.output ? "" : undefined}
    >
      <span>NDI® is a registered trademark of Vizrt NDI AB.</span>{" "}
      <NdiOfficialLinks />
    </p>
  );
}

export function NdiOfficialLinks() {
  return (
    <>
      <span>NDI® website:</span>{" "}
      <span class="ndiAttributionUrl" data-no-localize data-ndi-official-link="site">https://ndi.video/</span>
      {" · "}
      <span>Free NDI® Tools:</span>{" "}
      <span class="ndiAttributionUrl" data-no-localize data-ndi-official-link="tools">https://ndi.video/tools/</span>
    </>
  );
}
