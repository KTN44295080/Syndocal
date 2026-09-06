import type { Accessor, Setter } from "solid-js";
import {
  customProfileAttributeDraftsFromText,
  customProfileAttributeTextFromDrafts,
  type CustomProfileAttributeDraft,
} from "./customFixtureProfile";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { CustomFixtureProfileRequest, FixtureProfileSummary } from "./types";
import type { SetupSubTab } from "./uiModes";

export interface GdtfProfileActionContext {
  invoke: FrontendTauriInvoke;
  gdtfPath: Accessor<string>;
  setGdtfPath: Setter<string>;
  gdtfShareUrl: Accessor<string>;
  setLabel: Setter<string>;
  setMessage: (message: string) => unknown;
  profileLoadMessage: (prefix: string, profile: FixtureProfileSummary) => string;
  setProfile: Setter<FixtureProfileSummary | null>;
  setSelectedMode: Setter<string>;
  selectSetupMode: (tab: SetupSubTab) => void;
  focusPatchFixtureForm: () => void;
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  isProjectAuthorityIdentityCurrent: (captured: ProjectAuthorityToken) => boolean;
  customManufacturer: Accessor<string>;
  setCustomManufacturer: Setter<string>;
  customProfileName: Accessor<string>;
  setCustomProfileName: Setter<string>;
  customModeName: Accessor<string>;
  setCustomModeName: Setter<string>;
  customAttributes: Accessor<string>;
  setCustomAttributes: Setter<string>;
  customAttributeDrafts: Accessor<CustomProfileAttributeDraft[]>;
  setCustomAttributeDrafts: Setter<CustomProfileAttributeDraft[]>;
  selectedCustomAttributeIndexValue: Accessor<number | null>;
  setSelectedCustomAttributeIndex: Setter<number | null>;
}

export interface GdtfProfileActions {
  selectGdtfFile: () => Promise<void>;
  selectLoadedProfile: (
    imported: FixtureProfileSummary,
    loadedMessage: string,
    preferredMode?: string | null,
    openPatch?: boolean,
  ) => void;
  loadGdtfProfile: (
    path: string,
    loadedMessage?: string,
    preferredMode?: string | null,
    openPatch?: boolean,
  ) => Promise<void>;
  importGdtf: () => Promise<void>;
  downloadGdtfFromUrl: () => Promise<void>;
  setCustomAttributesText: (value: string) => void;
  commitCustomAttributeDrafts: (drafts: CustomProfileAttributeDraft[], selectedIndex?: number | null) => void;
  updateCustomAttributeDraft: (index: number, updates: Partial<CustomProfileAttributeDraft>) => void;
  addCustomAttributeDraft: () => void;
  appendCustomAttributeTemplate: (rows: CustomProfileAttributeDraft[]) => void;
  removeCustomAttributeDraft: (index: number) => void;
  moveCustomAttributeDraft: (index: number, delta: -1 | 1) => void;
  createCustomProfile: () => Promise<void>;
  saveCustomProfile: () => Promise<void>;
  loadCustomProfile: () => Promise<void>;
}

export function createGdtfProfileActions(context: GdtfProfileActionContext): GdtfProfileActions {
  const fixtureLabelForProfile = (profile: FixtureProfileSummary) =>
    profile.name.trim() || profile.manufacturer.trim() || "Fixture";

  const applyFixtureLabelForProfile = (profile: FixtureProfileSummary) => {
    context.setLabel(fixtureLabelForProfile(profile));
  };

  const selectGdtfFile = async () => {
    try {
      const path = await context.invoke<string | null>("select_gdtf_file");
      if (path) {
        context.setGdtfPath(path);
        context.setMessage(`Selected ${path}`);
      }
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  const selectLoadedProfile = (
    imported: FixtureProfileSummary,
    loadedMessage: string,
    preferredMode: string | null = null,
    openPatch = false,
  ) => {
    const modeName = preferredMode && imported.dmx_modes.some((mode) => mode.name === preferredMode)
      ? preferredMode
      : imported.dmx_modes[0]?.name ?? "";
    context.setProfile(imported);
    context.setGdtfPath(imported.source_path);
    context.setSelectedMode(modeName);
    applyFixtureLabelForProfile(imported);
    context.setMessage(context.profileLoadMessage(loadedMessage, imported));
    if (openPatch) {
      context.selectSetupMode("patch");
      context.focusPatchFixtureForm();
    }
  };

  const loadGdtfProfile = async (
    path: string,
    loadedMessage = "Loaded",
    preferredMode: string | null = null,
    openPatch = false,
  ) => {
    const imported = await context.invoke<FixtureProfileSummary>("import_gdtf", { path });
    selectLoadedProfile(imported, loadedMessage, preferredMode, openPatch);
  };

  const importGdtf = async () => {
    try {
      await loadGdtfProfile(context.gdtfPath(), "Loaded", null, false);
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  const downloadGdtfFromUrl = async () => {
    try {
      const path = await context.invoke<string | null>("download_gdtf_from_url", { url: context.gdtfShareUrl() });
      if (!path) {
        context.setMessage("GDTF download canceled.");
        return;
      }
      context.setGdtfPath(path);
      await loadGdtfProfile(path, "Downloaded and loaded", null, false);
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  const setCustomAttributesText = (value: string) => {
    context.setCustomAttributes(value);
    context.setCustomAttributeDrafts(customProfileAttributeDraftsFromText(value));
    context.setSelectedCustomAttributeIndex(null);
  };

  const commitCustomAttributeDrafts = (drafts: CustomProfileAttributeDraft[], selectedIndex?: number | null) => {
    context.setCustomAttributeDrafts(drafts);
    context.setCustomAttributes(customProfileAttributeTextFromDrafts(drafts));
    context.setSelectedCustomAttributeIndex(
      selectedIndex !== undefined
        ? selectedIndex
        : context.selectedCustomAttributeIndexValue() !== null
          && context.selectedCustomAttributeIndexValue()! < drafts.length
          ? context.selectedCustomAttributeIndexValue()
          : drafts.length > 0
            ? drafts.length - 1
            : null,
    );
  };

  const updateCustomAttributeDraft = (index: number, updates: Partial<CustomProfileAttributeDraft>) => {
    const drafts = context.customAttributeDrafts();
    if (index < 0 || index >= drafts.length) {
      return;
    }
    commitCustomAttributeDrafts(
      drafts.map((draft, draftIndex) => (draftIndex === index ? { ...draft, ...updates } : draft)),
      index,
    );
  };

  const addCustomAttributeDraft = () => {
    const drafts = context.customAttributeDrafts();
    commitCustomAttributeDrafts(
      [...drafts, { attribute: `Attribute${drafts.length + 1}`, resolution: "EightBit", startOffset: "" }],
      drafts.length,
    );
  };

  const appendCustomAttributeTemplate = (rows: CustomProfileAttributeDraft[]) => {
    const drafts = context.customAttributeDrafts();
    const nextRows = rows.map((row) => ({ ...row }));
    commitCustomAttributeDrafts([...drafts, ...nextRows], drafts.length);
  };

  const removeCustomAttributeDraft = (index: number) => {
    const drafts = context.customAttributeDrafts();
    if (index < 0 || index >= drafts.length) {
      return;
    }
    const next = drafts.filter((_, draftIndex) => draftIndex !== index);
    commitCustomAttributeDrafts(next, next.length === 0 ? null : Math.min(index, next.length - 1));
  };

  const moveCustomAttributeDraft = (index: number, delta: -1 | 1) => {
    const drafts = [...context.customAttributeDrafts()];
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || index >= drafts.length || nextIndex >= drafts.length) {
      return;
    }
    const [draft] = drafts.splice(index, 1);
    drafts.splice(nextIndex, 0, draft);
    commitCustomAttributeDrafts(drafts, nextIndex);
  };

  const customProfileRequest = (): CustomFixtureProfileRequest => ({
    manufacturer: context.customManufacturer(),
    name: context.customProfileName(),
    mode_name: context.customModeName(),
    attributes: context.customAttributes()
      .split(",")
      .map((attribute) => attribute.trim())
      .filter(Boolean),
  });

  const createCustomProfile = async () => {
    const request = customProfileRequest();
    const authority = context.captureProjectAuthorityIdentity();
    try {
      const created = await context.invoke<FixtureProfileSummary>("preview_custom_fixture_profile", { request });
      if (!context.isProjectAuthorityIdentityCurrent(authority)) {
        context.setMessage("Project changed while previewing the custom fixture profile; the preview was discarded.");
        return;
      }
      context.setProfile(created);
      context.setGdtfPath(created.source_path);
      context.setSelectedMode(created.dmx_modes[0]?.name ?? "");
      applyFixtureLabelForProfile(created);
      context.setMessage(context.profileLoadMessage("Created custom profile", created));
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  const saveCustomProfile = async () => {
    const request = customProfileRequest();
    try {
      const path = await context.invoke<string | null>("save_custom_fixture_profile", { request });
      context.setMessage(path ? `Saved custom profile ${path}` : "Custom profile save canceled.");
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  const loadCustomProfile = async () => {
    try {
      const created = await context.invoke<FixtureProfileSummary | null>("load_custom_fixture_profile");
      if (!created) {
        context.setMessage("Custom profile load canceled.");
        return;
      }
      context.setProfile(created);
      context.setGdtfPath(created.source_path);
      context.setSelectedMode(created.dmx_modes[0]?.name ?? "");
      applyFixtureLabelForProfile(created);
      context.setCustomManufacturer(created.manufacturer);
      context.setCustomProfileName(created.name);
      context.setCustomModeName(created.dmx_modes[0]?.name ?? "Default");
      setCustomAttributesText(
        created.dmx_modes[0]?.controls
          .map((control) => `${control.attribute}@${control.offsets[0] ?? 1}:${control.resolution === "SixteenBit" ? "16" : "8"}`)
          .join(", ") ?? "",
      );
      context.setMessage(context.profileLoadMessage("Loaded custom profile", created));
    } catch (error) {
      context.setMessage(String(error));
    }
  };

  return {
    selectGdtfFile,
    selectLoadedProfile,
    loadGdtfProfile,
    importGdtf,
    downloadGdtfFromUrl,
    setCustomAttributesText,
    commitCustomAttributeDrafts,
    updateCustomAttributeDraft,
    addCustomAttributeDraft,
    appendCustomAttributeTemplate,
    removeCustomAttributeDraft,
    moveCustomAttributeDraft,
    createCustomProfile,
    saveCustomProfile,
    loadCustomProfile,
  };
}
