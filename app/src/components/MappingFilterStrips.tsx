import { createSignal, For, onCleanup, Show } from "solid-js";
import { mappingTypeGlyphClass, type MappingFixtureVisualKind } from "../fixtureVisuals";
import { handleHorizontalWheel } from "../horizontalWheel";

export interface MappingGroupStripRow {
  groupId: string;
  label: string;
  color?: string | null;
  count: number;
}

export interface MappingFixtureTypeStripRow {
  key: string;
  label: string;
  manufacturer: string;
  visualKind: MappingFixtureVisualKind;
  count: number;
}

export type MappingFilterStripsProps = {
  fixtureCount: number;
  filteredFixtureCount: number;
  selectedGroupId: string | null;
  groupRows: MappingGroupStripRow[];
  selectedTypeKey: string | null;
  fixtureTypeRows: MappingFixtureTypeStripRow[];
  onSelectGroup: (groupId: string | null) => void;
  onCreateGroup: (label: string) => void | Promise<void>;
  onRenameGroup: (groupId: string, label: string) => void | Promise<void>;
  onDeleteGroup: (groupId: string) => void | Promise<void>;
  onRecolorGroup: (groupId: string, color: string) => void | Promise<void>;
  onSelectType: (typeKey: string | null) => void;
};

export type MappingGroupRibbonProps = Pick<
  MappingFilterStripsProps,
  | "fixtureCount"
  | "selectedGroupId"
  | "groupRows"
  | "onSelectGroup"
  | "onCreateGroup"
  | "onRenameGroup"
  | "onDeleteGroup"
  | "onRecolorGroup"
> & {
  controlChrome?: boolean;
};

export type MappingFixtureTypeStripProps = Pick<
  MappingFilterStripsProps,
  "filteredFixtureCount" | "selectedTypeKey" | "fixtureTypeRows" | "onSelectType"
>;

type GroupContextMenu = {
  groupId: string;
  x: number;
  y: number;
};

export function MappingGroupRibbon(props: MappingGroupRibbonProps) {
  const [editingGroupId, setEditingGroupId] = createSignal<string | null>(null);
  const [renameDraft, setRenameDraft] = createSignal("");
  const [creating, setCreating] = createSignal(false);
  const [createDraft, setCreateDraft] = createSignal("");
  const [contextMenu, setContextMenu] = createSignal<GroupContextMenu | null>(null);
  const [deleteTargetId, setDeleteTargetId] = createSignal<string | null>(null);
  let deleteDialog: HTMLDialogElement | undefined;

  const groupForId = (groupId: string) => props.groupRows.find((group) => group.groupId === groupId);
  const focusInput = (input: HTMLInputElement) => queueMicrotask(() => {
    input.focus();
    input.select();
  });
  const beginRename = (groupId: string) => {
    const group = groupForId(groupId);
    if (!group) return;
    setContextMenu(null);
    setCreating(false);
    setRenameDraft(group.label);
    setEditingGroupId(groupId);
  };
  const commitRename = async () => {
    const groupId = editingGroupId();
    const label = renameDraft().trim();
    if (!groupId) return;
    setEditingGroupId(null);
    if (label && label !== groupForId(groupId)?.label) {
      await props.onRenameGroup(groupId, label);
    }
  };
  const commitCreate = async () => {
    const label = createDraft().trim();
    setCreating(false);
    setCreateDraft("");
    if (label) await props.onCreateGroup(label);
  };
  const requestDelete = (groupId: string) => {
    setContextMenu(null);
    setDeleteTargetId(groupId);
    if (!deleteDialog?.open) deleteDialog?.showModal();
  };
  const confirmDelete = async () => {
    const groupId = deleteTargetId();
    deleteDialog?.close();
    setDeleteTargetId(null);
    if (groupId) await props.onDeleteGroup(groupId);
  };
  const closeContextMenu = (event: PointerEvent) => {
    if ((event.target as Element | null)?.closest("[data-group-context-menu]")) return;
    setContextMenu(null);
  };
  const closeContextMenuFromKey = (event: KeyboardEvent) => {
    if (
      event.key !== "Escape" ||
      event.defaultPrevented ||
      !contextMenu() ||
      document.querySelector("dialog[open]")
    ) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    setContextMenu(null);
  };
  window.addEventListener("pointerdown", closeContextMenu, true);
  window.addEventListener("keydown", closeContextMenuFromKey, true);
  onCleanup(() => {
    window.removeEventListener("pointerdown", closeContextMenu, true);
    window.removeEventListener("keydown", closeContextMenuFromKey, true);
  });

  return (
    <div
      class="mappingGroupStrip"
      data-persistent-band-part="groups"
      data-control-stage-chrome-operation={props.controlChrome ? "groups" : undefined}
      data-wheel-scroll-surface="group-chips"
      onWheel={handleHorizontalWheel}
    >
      <span>Groups</span>
      <button
        type="button"
        class={!props.selectedGroupId ? "active" : ""}
        onClick={() => props.onSelectGroup(null)}
      >
        All
        <small>{props.fixtureCount}</small>
      </button>
      <For each={props.groupRows}>
        {(group) => (
          <Show
            when={editingGroupId() === group.groupId}
            fallback={
              <button
                type="button"
                class={props.selectedGroupId === group.groupId ? "active" : ""}
                data-group-tab={group.groupId}
                style={{ "--fixture-group-color": group.color ?? "#5f6b76" }}
                onClick={() => props.onSelectGroup(group.groupId)}
                onDblClick={() => beginRename(group.groupId)}
                onContextMenu={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  setContextMenu({
                    groupId: group.groupId,
                    x: Math.min(event.clientX, window.innerWidth - 184),
                    y: Math.min(event.clientY, window.innerHeight - 150),
                  });
                }}
              >
                <span data-no-localize>{group.label}</span>
                <small>{group.count}</small>
              </button>
            }
          >
            <input
              ref={focusInput}
              class="mappingGroupInlineRename"
              data-group-inline-rename={group.groupId}
              aria-label="Rename group"
              value={renameDraft()}
              onInput={(event) => setRenameDraft(event.currentTarget.value)}
              onBlur={() => void commitRename()}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void commitRename();
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  setEditingGroupId(null);
                }
              }}
            />
          </Show>
        )}
      </For>
      <Show when={creating()}>
        <input
          ref={focusInput}
          class="mappingGroupInlineRename"
          data-group-inline-create
          aria-label="New group name"
          placeholder="Group name"
          value={createDraft()}
          onInput={(event) => setCreateDraft(event.currentTarget.value)}
          onBlur={() => void commitCreate()}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void commitCreate();
            } else if (event.key === "Escape") {
              event.preventDefault();
              setCreating(false);
              setCreateDraft("");
            }
          }}
        />
      </Show>
      <button
        type="button"
        class="mappingGroupCreateButton"
        data-group-create
        aria-label="Create group"
        title="Create group"
        onClick={() => {
          setContextMenu(null);
          setEditingGroupId(null);
          setCreating(true);
        }}
      >
        +
      </button>
      <Show when={contextMenu()}>
        {(menu) => {
          const group = () => groupForId(menu().groupId);
          return (
            <div
              class="mappingGroupContextMenu"
              data-group-context-menu={menu().groupId}
              role="menu"
              aria-label="Group actions"
              style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
            >
              <button type="button" role="menuitem" onClick={() => beginRename(menu().groupId)}>
                Rename
              </button>
              <label>
                <span>Color</span>
                <input
                  type="color"
                  aria-label="Group color"
                  value={group()?.color ?? "#5f6b76"}
                  onChange={(event) => {
                    setContextMenu(null);
                    void props.onRecolorGroup(menu().groupId, event.currentTarget.value);
                  }}
                />
              </label>
              <button
                type="button"
                class="danger"
                role="menuitem"
                onClick={() => requestDelete(menu().groupId)}
              >
                Delete
              </button>
            </div>
          );
        }}
      </Show>
      <dialog
        ref={deleteDialog}
        class="mappingGroupDeleteDialog"
        data-group-delete-dialog
        aria-labelledby="group-delete-title"
        onClose={() => setDeleteTargetId(null)}
      >
        <form method="dialog" onSubmit={(event) => event.preventDefault()}>
          <h3 id="group-delete-title">Delete group?</h3>
          <p>
            Remove {groupForId(deleteTargetId() ?? "")?.label ?? "this group"} from all fixtures?
          </p>
          <div class="dialogActions">
            <button type="button" onClick={() => deleteDialog?.close()}>Cancel</button>
            <button type="button" class="danger" onClick={() => void confirmDelete()}>Delete</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}

export function MappingFixtureTypeStrip(props: MappingFixtureTypeStripProps) {
  return (
    <div
      class="mappingTypeStrip"
      data-wheel-scroll-surface="fixture-type-chips"
      onWheel={handleHorizontalWheel}
    >
      <span>Types</span>
      <button
        type="button"
        class={!props.selectedTypeKey ? "active" : ""}
        aria-label="All fixture types"
        title="Show all fixture types"
        onClick={() => props.onSelectType(null)}
      >
        All Types
        <small>{props.filteredFixtureCount}</small>
      </button>
      <For each={props.fixtureTypeRows}>
        {(row) => (
          <button
            type="button"
            class={props.selectedTypeKey === row.key ? "active" : ""}
            aria-label={`Filter by fixture type: ${row.label}`}
            onClick={() => props.onSelectType(row.key)}
            title={`${row.manufacturer} ${row.label}`}
          >
            <span class={mappingTypeGlyphClass(row.visualKind)} />
            <span data-no-localize>{row.label}</span>
            <small>{row.count}</small>
          </button>
        )}
      </For>
    </div>
  );
}

export function MappingFilterStrips(props: MappingFilterStripsProps) {
  return (
    <>
      <MappingGroupRibbon
        fixtureCount={props.fixtureCount}
        selectedGroupId={props.selectedGroupId}
        groupRows={props.groupRows}
        onSelectGroup={props.onSelectGroup}
        onCreateGroup={props.onCreateGroup}
        onRenameGroup={props.onRenameGroup}
        onDeleteGroup={props.onDeleteGroup}
        onRecolorGroup={props.onRecolorGroup}
      />
      <MappingFixtureTypeStrip
        filteredFixtureCount={props.filteredFixtureCount}
        selectedTypeKey={props.selectedTypeKey}
        fixtureTypeRows={props.fixtureTypeRows}
        onSelectType={props.onSelectType}
      />
    </>
  );
}
