import { For, onMount, Show } from "solid-js";
import type { DvcImportCategory, DvcImportReport } from "../types";

type DvcImportReportPanelProps = {
  report: DvcImportReport;
  onClose: () => void;
};

type ReportCategory = {
  label: string;
  value: DvcImportCategory;
  tone: "converted" | "approximate" | "skipped" | "unsupported";
};

export function DvcImportReportPanel(props: DvcImportReportPanelProps) {
  let reportDialog!: HTMLDialogElement;
  const categories = (): ReportCategory[] => [
    { label: "Converted", value: props.report.converted, tone: "converted" },
    { label: "Approximate", value: props.report.approximate, tone: "approximate" },
    { label: "Skipped", value: props.report.skipped, tone: "skipped" },
    { label: "Unsupported", value: props.report.unsupported, tone: "unsupported" },
  ];

  const summary = () => [
    ["Fixtures", props.report.summary.fixtures],
    ["Profiles", props.report.summary.profiles],
    ["Fixture groups", props.report.summary.fixture_groups],
    ["Scene banks", props.report.summary.groups],
    ["Cues", props.report.summary.cues],
    ["Value payloads converted", props.report.summary.values_decoded],
    ["Value payloads skipped", props.report.summary.values_skipped],
    ["Beam feature records", props.report.summary.beam_records],
    ["Beam feature mismatches", props.report.summary.beam_feature_mismatches],
    ["Audio clips", props.report.summary.timeline_audio_clips],
    ["Scene Blocks", props.report.summary.timeline_scene_blocks],
    ["Effects converted", props.report.summary.effects_converted],
    ["Effects skipped", props.report.summary.effects_skipped],
    ["Unknown channel types", props.report.summary.unknown_channel_types],
    ["Missing audio files", props.report.summary.missing_audio_files],
  ] as const;

  onMount(() => reportDialog.showModal());

  return (
    <dialog
      ref={reportDialog}
      class="dvcImportReportDialog"
      aria-labelledby="dvc-import-report-title"
      onClose={props.onClose}
    >
      <section
        class="dvcImportReportPanel"
      >
        <header class="dvcImportReportHeader">
          <div>
            <p>Daslight Project (.dvc)</p>
            <h2 id="dvc-import-report-title" class="textBalance">Import report</h2>
          </div>
          <button type="button" aria-label="Close import report" onClick={() => reportDialog.close()}>
            Close
          </button>
        </header>

        <div class="dvcImportReportBody">
          <div class="dvcImportReportSource">
            <span>Source</span>
            <strong data-no-localize>{props.report.path}</strong>
            <small>
              Daslight build {props.report.das_build || "unknown"} · VERSIONFILE {props.report.version_file || "unknown"}
            </small>
          </div>

          <p class="dvcImportSafetyNote textPretty">Imported DMX routes remain disabled. Save As to create a Syndocal Project (.sdc).</p>

          <dl class="dvcImportSummaryGrid">
            <For each={summary()}>
              {([label, count]) => (
                <div>
                  <dt>{label}</dt>
                  <dd class="tabularNums" data-no-localize>{count}</dd>
                </div>
              )}
            </For>
          </dl>

          <Show when={props.report.warnings.length > 0}>
            <section class="dvcImportWarnings">
              <h3>Warnings</h3>
              <ul>
                <For each={props.report.warnings}>{(warning) => <li>{warning}</li>}</For>
              </ul>
            </section>
          </Show>

          <div class="dvcImportCategoryList">
            <For each={categories()}>
              {(category) => (
                <section class={`dvcImportCategory ${category.tone}`}>
                  <header>
                    <h3>{category.label}</h3>
                    <span class="tabularNums" data-no-localize>{category.value.count}</span>
                  </header>
                  <Show
                    when={category.value.details.length > 0}
                    fallback={<p class="dvcImportEmptyCategory">No items</p>}
                  >
                    <ul>
                      <For each={category.value.details}>
                        {(detail) => (
                          <li>
                            <strong>{detail.item}</strong>
                            <span>{detail.message}</span>
                          </li>
                        )}
                      </For>
                    </ul>
                  </Show>
                </section>
              )}
            </For>
          </div>
        </div>
      </section>
    </dialog>
  );
}
