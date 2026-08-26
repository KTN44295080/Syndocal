import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { groupIdentityCss, groupIdentityHue } from "../identityColor";
import { handleHorizontalWheel } from "../horizontalWheel";
import { displayNumber } from "../numberDisplay";
import { sceneCueKind } from "../sceneCueKind";
import type {
  ActiveFadeSummary,
  CueLiveDirection,
  CueLiveModifierState,
  CueListSummary,
  CueSummary,
  TimelineTrackKind,
} from "../types";
import type { TimelineCueDragPoint } from "../timelineCueDrag";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import { CueLiveModifierStrip } from "./CueLiveModifierStrip";
import { controlMappingTargetData } from "../controlMappingLearn";
import { bankAuthorityIssueMessage, type FullBankAuthoritySnapshot } from "../bankAuthority";

interface SceneMatrixPanelProps {
  toolbar?: JSX.Element;
  bankAuthority: FullBankAuthoritySnapshot;
  /** Authoritative backend rejection text for a failed Bank create/rename. */
  bankMutationFailureMessage?: string;
  selectedCueListId: number | null;
  onSelectCueList: (cueListId: number) => void;
  onCueListLabel: (label: string) => void;
  onCreateCueList: () => void | boolean | Promise<void | boolean>;
  onRenameCueList: () => void | boolean | Promise<void | boolean>;
  onRemoveCueList?: (cueListId?: number, confirmed?: boolean) => void | boolean | Promise<void | boolean>;
  onReorderCueLists?: (orderedCueListIds: number[]) => void | boolean | Promise<void | boolean>;
  onCreateSceneForCueList?: (cueListId: number) => void | boolean | Promise<void | boolean>;
  onRenameCue?: (cueId: number) => void | Promise<void>;
  onDuplicateCue?: (cue: CueSummary) => void | Promise<void>;
  onRemoveCue?: (cueId: number, confirmed?: boolean) => void | boolean | Promise<void | boolean>;
  cueRemovalImpact?: (cueId: number) => {
    linkedPlacements: number;
    incomingJumps: number;
  };
  groupColors?: Record<string, string>;
  onSetGroupColor?: (groupId: string, color: string | null) => void | Promise<void>;
  groupIds: string[];
  activeCueId: number | null | undefined;
  activeGroupCueIds: Record<string, number>;
  selectedCueId?: number | null;
  activeFade?: ActiveFadeSummary | null;
  cueLiveModifiers?: CueLiveModifierState[];
  onSetCueLiveModifier?: (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => void | Promise<void>;
  onClearCueLiveModifier?: (cueId: number) => void | Promise<void>;
  onReleaseCue: (cueId: number) => void | Promise<void>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onSelectCue: (cueId: number) => void;
  onOpenSuperScene: (cueId: number) => void | Promise<void>;
  onOpenCueEditor: () => void;
  onBeginTimelineCueDrag: (
    cue: CueSummary,
    point: TimelineCueDragPoint,
    sourceSurface?: "scene-matrix",
  ) => void;
  onMoveTimelineCueDrag: (point: TimelineCueDragPoint) => void;
  onEndTimelineCueDrag: (point: TimelineCueDragPoint, moved: boolean, canceled: boolean) => void;
  timelineTrack: TimelineTrackKind;
}

interface SceneMatrixColumn {
  id: string;
  cueListId: number;
  label: string;
  cues: CueSummary[];
}

const SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX = 4;

export function SceneMatrixPanel(props: SceneMatrixPanelProps) {
  const [dragCueId, setDragCueId] = createSignal<number | null>(null);
  const [activeBankId, setActiveBankId] = createSignal("Show");
  const [bankEditorMode, setBankEditorMode] = createSignal<"create" | "rename" | null>(null);
  const [bankDraft, setBankDraft] = createSignal("");
  const [bankEditorError, setBankEditorError] = createSignal<string | null>(null);
  const [bankEditorSubmitting, setBankEditorSubmitting] = createSignal(false);
  const [bankContextMenu, setBankContextMenu] = createSignal<{
    cueListId: number;
    left: number;
    top: number;
  } | null>(null);
  const [sceneContextMenu, setSceneContextMenu] = createSignal<{
    cueId: number;
    left: number;
    top: number;
  } | null>(null);
  const [bankDeleteRequest, setBankDeleteRequest] = createSignal<CueListSummary | null>(null);
  const [sceneDeleteRequest, setSceneDeleteRequest] = createSignal<CueSummary | null>(null);
  const [bankDragId, setBankDragId] = createSignal<number | null>(null);
  const [bankDropTargetId, setBankDropTargetId] = createSignal<number | null>(null);
  const [dropTargetColumnId, setDropTargetColumnId] = createSignal<string | null>(null);
  const [dropIndicator, setDropIndicator] = createSignal<{
    cueId: number;
    position: "before" | "after";
  } | null>(null);
  let scrollerElement: HTMLDivElement | undefined;
  let bankInputElement: HTMLInputElement | undefined;
  let bankContextMenuElement: HTMLDivElement | undefined;
  let bankContextMenuInvoker: HTMLElement | null = null;
  let sceneContextMenuElement: HTMLDivElement | undefined;
  let sceneContextMenuInvoker: HTMLElement | null = null;
  let bankDeleteDialogElement: HTMLDialogElement | undefined;
  let sceneDeleteDialogElement: HTMLDialogElement | undefined;
  let preferredBankId: string | null = null;
  let dragPointer: {
    pointerId: number;
    startClientX: number;
    startClientY: number;
    moved: boolean;
    cue: CueSummary;
  } | null = null;
  let suppressClickCueId: number | null = null;
  let sceneMatrixMounted = true;
  onCleanup(() => {
    sceneMatrixMounted = false;
  });

  // Engine snapshots deserialize every Cue into a fresh object. Keep the
  // rendered Cue objects keyed by id so a polling-only snapshot cannot remount
  // an active card and dismiss its native select/input interaction.
  const [stableCues, setStableCues] = createStore<CueSummary[]>(
    props.bankAuthority.issue === null ? [...props.bankAuthority.cues] : [],
  );
  createEffect(() => {
    setStableCues(reconcile(
      props.bankAuthority.issue === null ? [...props.bankAuthority.cues] : [],
      { key: "id" },
    ));
  });

  // Persisted Bank labels are the display authority. Never rewrite an authored
  // `Main` label to `Bank 1` in only one surface.
  const cueLists = createMemo<readonly CueListSummary[]>(() => props.bankAuthority.issue
    ? []
    : props.bankAuthority.cueLists);
  const selectedCueList = createMemo<CueListSummary | null>(() =>
    cueLists().find((cueList) => cueList.id === props.selectedCueListId)
      ?? null,
  );
  createEffect(() => {
    if (!props.bankAuthority.issue) return;
    // Keep an already-open create/rename form visible. Its authoritative
    // callback can reject for this operator-lock transition, and closing it
    // would discard both the draft and the actionable inline failure.
    setBankContextMenu(null);
    setSceneContextMenu(null);
    setBankDeleteRequest(null);
    setSceneDeleteRequest(null);
    setBankDragId(null);
    setBankDropTargetId(null);
    setDropTargetColumnId(null);
    setDropIndicator(null);
  });
  const cancelBankEditor = () => {
    if (bankEditorSubmitting()) {
      if (sceneMatrixMounted) bankInputElement?.focus();
      return;
    }
    const selected = selectedCueList();
    if (bankEditorMode() === "rename" && selected) props.onCueListLabel(selected.label);
    setBankEditorMode(null);
    setBankDraft("");
    setBankEditorError(null);
  };
  const selectCueList = (cueListId: number) => {
    if (bankEditorMode()) cancelBankEditor();
    props.onSelectCueList(cueListId);
    preferredBankId = null;
    setActiveBankId(String(cueListId));
  };
  const nextAutomaticBankLabel = () => {
    const labels = new Set(cueLists().map((cueList) => cueList.label.trim().toLowerCase()));
    let index = 1;
    while (labels.has(`bank ${index}`)) index += 1;
    return `Bank ${index}`;
  };
  const beginCreateBank = () => {
    if (props.bankAuthority.issue) return;
    setBankEditorMode("create");
    setBankDraft(nextAutomaticBankLabel());
    setBankEditorError(null);
    setBankContextMenu(null);
  };
  const beginRenameBank = () => {
    const selected = selectedCueList();
    if (!selected) return;
    setBankEditorMode("rename");
    setBankDraft(selected.label);
    setBankEditorError(null);
    setBankContextMenu(null);
  };
  const requestContextMenu = (event: MouseEvent | KeyboardEvent, cueListId: number) => {
    event.preventDefault();
    const bank = cueLists().find((cueList) => cueList.id === cueListId);
    if (!bank) return;
    const anchor = event.currentTarget instanceof HTMLElement
      ? event.currentTarget.getBoundingClientRect()
      : null;
    const pointerX = event instanceof MouseEvent
      ? event.clientX
      : anchor?.left ?? 0;
    const pointerY = event instanceof MouseEvent
      ? event.clientY
      : anchor?.bottom ?? 0;
    selectCueList(cueListId);
    bankContextMenuInvoker = event.currentTarget instanceof HTMLElement
      ? event.currentTarget
      : null;
    closeSceneContextMenu(false);
    setBankContextMenu({
      cueListId,
      left: Math.max(8, Math.min(pointerX, window.innerWidth - 188)),
      top: Math.max(8, Math.min(pointerY, window.innerHeight - 164)),
    });
    queueMicrotask(() => {
      const menu = bankContextMenuElement;
      const target = bankContextMenu();
      if (menu && target) {
        const rect = menu.getBoundingClientRect();
        setBankContextMenu({
          ...target,
          left: Math.max(8, Math.min(target.left, window.innerWidth - rect.width - 8)),
          top: Math.max(8, Math.min(target.top, window.innerHeight - rect.height - 8)),
        });
      }
      menu?.querySelector<HTMLButtonElement>("[role=menuitem]:not(:disabled)")?.focus();
    });
  };
  const closeContextMenu = (restoreFocus = true) => {
    const invoker = bankContextMenuInvoker;
    bankContextMenuInvoker = null;
    setBankContextMenu(null);
    if (restoreFocus) queueMicrotask(() => invoker?.focus());
  };
  const beginContextRename = () => {
    const target = bankContextMenu();
    if (!target) return;
    selectCueList(target.cueListId);
    closeContextMenu(false);
    beginRenameBank();
  };
  const createContextScene = () => {
    const target = bankContextMenu();
    if (!target) return;
    closeContextMenu(false);
    createScene(target.cueListId);
  };
  const requestContextDelete = () => {
    const target = bankContextMenu();
    if (!target) return;
    const bank = cueLists().find((cueList) => cueList.id === target.cueListId);
    closeContextMenu();
    if (!bank || cueLists().length <= 1 || !props.onRemoveCueList) return;
    selectCueList(bank.id);
    setBankDeleteRequest(bank);
  };
  const confirmContextDelete = async () => {
    const target = bankDeleteRequest();
    if (!target || !props.onRemoveCueList) return;
    const result = await props.onRemoveCueList(target.id, true);
    // Keep the confirmation open when the authoritative backend rejects or
    // loses the mutation. Closing here would falsely imply that the Bank and
    // its scenes were deleted and would also discard the user's retry path.
    if (result === false) {
      queueMicrotask(() => bankDeleteDialogElement
        ?.querySelector<HTMLButtonElement>("[data-scene-matrix-delete-confirm]")
        ?.focus());
      return;
    }
    setBankDeleteRequest(null);
    bankDeleteDialogElement?.close();
  };
  const cancelContextDelete = () => {
    setBankDeleteRequest(null);
    bankDeleteDialogElement?.close();
  };
  const requestSceneContextMenu = (event: MouseEvent | KeyboardEvent, cue: CueSummary) => {
    event.preventDefault();
    const anchor = event.currentTarget instanceof HTMLElement
      ? event.currentTarget.getBoundingClientRect()
      : null;
    const pointerX = event instanceof MouseEvent ? event.clientX : anchor?.left ?? 0;
    const pointerY = event instanceof MouseEvent ? event.clientY : anchor?.bottom ?? 0;
    closeContextMenu(false);
    props.onSelectCue(cue.id);
    sceneContextMenuInvoker = event.currentTarget instanceof HTMLElement
      ? event.currentTarget
      : null;
    setSceneContextMenu({
      cueId: cue.id,
      left: Math.max(8, Math.min(pointerX, window.innerWidth - 188)),
      top: Math.max(8, Math.min(pointerY, window.innerHeight - 212)),
    });
    queueMicrotask(() => {
      const menu = sceneContextMenuElement;
      const target = sceneContextMenu();
      if (menu && target) {
        const rect = menu.getBoundingClientRect();
        setSceneContextMenu({
          ...target,
          left: Math.max(8, Math.min(target.left, window.innerWidth - rect.width - 8)),
          top: Math.max(8, Math.min(target.top, window.innerHeight - rect.height - 8)),
        });
      }
      menu?.querySelector<HTMLButtonElement>("[role=menuitem]:not(:disabled)")?.focus();
    });
  };
  const closeSceneContextMenu = (restoreFocus = true) => {
    const invoker = sceneContextMenuInvoker;
    sceneContextMenuInvoker = null;
    setSceneContextMenu(null);
    if (restoreFocus) queueMicrotask(() => invoker?.focus());
  };
  const sceneContextCue = () => {
    const target = sceneContextMenu();
    return target ? stableCues.find((cue) => cue.id === target.cueId) ?? null : null;
  };
  const renameContextScene = () => {
    const cue = sceneContextCue();
    closeSceneContextMenu(false);
    if (cue && props.onRenameCue) void props.onRenameCue(cue.id);
  };
  const duplicateContextScene = () => {
    const cue = sceneContextCue();
    closeSceneContextMenu(false);
    if (cue && props.onDuplicateCue) void props.onDuplicateCue(cue);
  };
  const openContextSceneTimeline = () => {
    const cue = sceneContextCue();
    closeSceneContextMenu(false);
    if (cue?.child_timeline) void props.onOpenSuperScene(cue.id);
  };
  const requestContextSceneDelete = () => {
    const cue = sceneContextCue();
    closeSceneContextMenu(false);
    if (cue && props.onRemoveCue) setSceneDeleteRequest(cue);
  };
  const confirmContextSceneDelete = async () => {
    const cue = sceneDeleteRequest();
    if (!cue || !props.onRemoveCue) return;
    const result = await props.onRemoveCue(cue.id, true);
    if (result === false) {
      queueMicrotask(() => sceneDeleteDialogElement
        ?.querySelector<HTMLButtonElement>("[data-scene-matrix-scene-delete-confirm]")
        ?.focus());
      return;
    }
    setSceneDeleteRequest(null);
    sceneDeleteDialogElement?.close();
  };
  const cancelContextSceneDelete = () => {
    setSceneDeleteRequest(null);
    sceneDeleteDialogElement?.close();
  };
  const handleContextMenuKeyDown = (
    event: KeyboardEvent & { currentTarget: HTMLDivElement },
    close: (restoreFocus?: boolean) => void,
  ) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "Tab") {
      window.setTimeout(() => close(false), 0);
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>(
      '[role="menuitem"]:not(:disabled)',
    )];
    if (items.length === 0) return;
    event.preventDefault();
    const current = Math.max(0, items.indexOf(document.activeElement as HTMLButtonElement));
    const next = event.key === "Home"
      ? 0
      : event.key === "End"
        ? items.length - 1
        : (current + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
    items[next]?.focus();
  };
  const createScene = (cueListId: number) => {
    props.onSelectCueList(cueListId);
    if (props.onCreateSceneForCueList) void props.onCreateSceneForCueList(cueListId);
    else void props.onOpenCueEditor();
  };
  const reorderCueLists = async (sourceId: number, targetId: number) => {
    if (sourceId === targetId || !props.onReorderCueLists) return;
    const ordered = cueLists().map((cueList) => cueList.id);
    const sourceIndex = ordered.indexOf(sourceId);
    const targetIndex = ordered.indexOf(targetId);
    if (sourceIndex < 0 || targetIndex < 0) return;
    ordered.splice(sourceIndex, 1);
    ordered.splice(ordered.indexOf(targetId), 0, sourceId);
    setBankDragId(null);
    setBankDropTargetId(null);
    await props.onReorderCueLists(ordered);
  };
  const submitBankEditor = async (event: SubmitEvent) => {
    event.preventDefault();
    if (bankEditorSubmitting()) return;
    const label = bankDraft().trim();
    if (!label) {
      setBankEditorError("Bank name is required.");
      bankInputElement?.focus();
      return;
    }
    const selected = selectedCueList();
    if (bankEditorMode() === "rename" && !selected) {
      setBankEditorError("The selected Bank is no longer available.");
      return;
    }
    const editingId = bankEditorMode() === "rename" ? selected!.id : null;
    const duplicate = cueLists().some((cueList) =>
      cueList.id !== editingId && cueList.label.trim().toLowerCase() === label.toLowerCase(),
    );
    if (duplicate) {
      setBankEditorError("That Bank name is already in use.");
      bankInputElement?.focus();
      bankInputElement?.select();
      return;
    }
    const mode = bankEditorMode();
    if (mode === null) {
      setBankEditorError("Bank editing is no longer available.");
      bankInputElement?.focus();
      return;
    }
    const failureMessage = (error?: unknown) => {
      const authoritative = props.bankMutationFailureMessage?.trim();
      if (authoritative) return authoritative;
      const operationSpecific = mode === "create"
        ? "Unable to create this Bank. Please try again."
        : "Unable to rename this Bank. Please try again.";
      if (operationSpecific) return operationSpecific;
      return String(error);
    };
    setBankEditorError(null);
    setBankEditorSubmitting(true);
    try {
      props.onCueListLabel(label);
      if (!sceneMatrixMounted) return;
      if (mode === "create") {
        const result = await props.onCreateCueList();
        if (!sceneMatrixMounted) return;
        if (result === false) {
          setBankEditorError(failureMessage());
          if (!sceneMatrixMounted) return;
          bankInputElement?.focus();
          return;
        }
      } else {
        const result = await props.onRenameCueList();
        if (!sceneMatrixMounted) return;
        if (result === false) {
          setBankEditorError(failureMessage());
          if (!sceneMatrixMounted) return;
          bankInputElement?.focus();
          return;
        }
      }
      setBankEditorMode(null);
      if (!sceneMatrixMounted) return;
      setBankDraft("");
      if (!sceneMatrixMounted) return;
      setBankEditorError(null);
    } catch (error) {
      if (!sceneMatrixMounted) return;
      setBankEditorError(failureMessage(error));
      if (!sceneMatrixMounted) return;
      bankInputElement?.focus();
    } finally {
      if (sceneMatrixMounted) setBankEditorSubmitting(false);
    }
  };
  createEffect(() => {
    if (bankEditorMode()) queueMicrotask(() => {
      if (!sceneMatrixMounted) return;
      bankInputElement?.focus();
      bankInputElement?.select();
    });
  });

  const calculatedColumns = createMemo<SceneMatrixColumn[]>(() => {
    return cueLists().map((cueList) => ({
      id: String(cueList.id),
      cueListId: cueList.id,
      label: cueList.label,
      cues: stableCues.filter((cue) => cue.cue_list_id === cueList.id),
    }));
  });
  const [stableColumns, setStableColumns] = createStore<SceneMatrixColumn[]>([]);
  createEffect(() => {
    setStableColumns(reconcile(calculatedColumns(), { key: "id" }));
  });
  const columns = () => stableColumns;

  const syncVisibleBank = () => {
    if (!scrollerElement) return;
    const scrollerRect = scrollerElement.getBoundingClientRect();
    if (preferredBankId) {
      const preferredColumn = [...scrollerElement.querySelectorAll<HTMLElement>(
        "[data-scene-matrix-column]",
      )].find((column) => column.dataset.sceneMatrixColumn === preferredBankId);
      if (preferredColumn) {
        const preferredRect = preferredColumn.getBoundingClientRect();
        const preferredVisibleWidth = Math.max(
          0,
          Math.min(preferredRect.right, scrollerRect.right) -
            Math.max(preferredRect.left, scrollerRect.left),
        );
        if (preferredVisibleWidth >= 2) {
          setActiveBankId(preferredBankId);
          return;
        }
      }
      preferredBankId = null;
    }
    let bestColumnId = columns()[0]?.id ?? "";
    let bestVisibleWidth = -1;
    for (const column of scrollerElement.querySelectorAll<HTMLElement>("[data-scene-matrix-column]")) {
      const rect = column.getBoundingClientRect();
      const visibleWidth = Math.max(
        0,
        Math.min(rect.right, scrollerRect.right) - Math.max(rect.left, scrollerRect.left),
      );
      if (visibleWidth > bestVisibleWidth) {
        bestVisibleWidth = visibleWidth;
        bestColumnId = column.dataset.sceneMatrixColumn ?? "Show";
      }
    }
    setActiveBankId(bestColumnId);
  };

  const jumpToBank = (columnId: string) => {
    if (!scrollerElement) return;
    const target = [...scrollerElement.querySelectorAll<HTMLElement>("[data-scene-matrix-column]")]
      .find((column) => column.dataset.sceneMatrixColumn === columnId);
    if (!target) return;
    preferredBankId = columnId;
    scrollerElement.scrollTo({
      left: Math.max(0, target.offsetLeft - 3),
      behavior: "auto",
    });
    setActiveBankId(columnId);
    window.requestAnimationFrame(syncVisibleBank);
  };

  const handleMatrixWheel = (event: WheelEvent & { currentTarget: HTMLDivElement }) => {
    if (bankContextMenu()) closeContextMenu(false);
    if (sceneContextMenu()) closeSceneContextMenu(false);
    const columnScroller = event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-scene-matrix-column-scroll]")
      : null;
    const verticalWheelDominant = Math.abs(event.deltaY) >= Math.abs(event.deltaX);
    if (
      !event.shiftKey &&
      verticalWheelDominant &&
      columnScroller &&
      columnScroller.scrollHeight > columnScroller.clientHeight + 1
    ) {
      return;
    }
    handleHorizontalWheel(event);
  };

  onMount(() => {
    syncVisibleBank();
    const resizeObserver = new ResizeObserver(syncVisibleBank);
    if (scrollerElement) resizeObserver.observe(scrollerElement);
    const closeMenuOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (bankContextMenuElement?.contains(target) || sceneContextMenuElement?.contains(target)) return;
      const targetElement = target instanceof Element ? target : target.parentElement;
      if (
        event.button === 2 &&
        targetElement?.closest("[data-scene-matrix-bank-jump], [data-scene-matrix-column-header]")
      ) return;
      if (bankContextMenu()) {
        const focusableOutside = targetElement?.closest(
          "button, a[href], input, select, textarea, [tabindex]:not([tabindex='-1'])",
        );
        closeContextMenu(!focusableOutside);
      }
      if (sceneContextMenu()) {
        const focusableOutside = targetElement?.closest(
          "button, a[href], input, select, textarea, [tabindex]:not([tabindex='-1'])",
        );
        closeSceneContextMenu(!focusableOutside);
      }
    };
    const closeMenuOnAncestorScroll = (event: Event) => {
      const target = event.target;
      // Context menus are fixed to their opening point. Their own content is
      // not a scrollport, but keep the exclusion symmetrical with Timeline so
      // a future bounded menu body cannot be mistaken for an outside scroll.
      if (target instanceof Node && (bankContextMenuElement?.contains(target) || sceneContextMenuElement?.contains(target))) {
        return;
      }
      if (bankContextMenu()) closeContextMenu(false);
      if (sceneContextMenu()) closeSceneContextMenu(false);
    };
    document.addEventListener("pointerdown", closeMenuOnOutsidePointer);
    window.addEventListener("scroll", closeMenuOnAncestorScroll, { capture: true, passive: true });
    onCleanup(() => {
      resizeObserver.disconnect();
      document.removeEventListener("pointerdown", closeMenuOnOutsidePointer);
      window.removeEventListener("scroll", closeMenuOnAncestorScroll, { capture: true });
    });
  });

  const beginDrag = (event: PointerEvent & { currentTarget: HTMLElement }, cue: CueSummary) => {
    if (event.button !== 0 || !event.isPrimary) return;
    dragPointer = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      moved: false,
      cue,
    };
  };

  const updateDropIndicator = (
    event: PointerEvent & { currentTarget: HTMLElement },
  ) => {
    if (!dragPointer?.moved) {
      setDropIndicator(null);
      setDropTargetColumnId(null);
      return;
    }
    const sourceCard = event.currentTarget.closest<HTMLElement>("[data-scene-matrix-cue-id]");
    const hitElement = document.elementFromPoint(event.clientX, event.clientY);
    const targetCard = hitElement
      ?.closest<HTMLElement>("[data-scene-matrix-cue-id]");
    const sourceColumn = sourceCard?.closest<HTMLElement>("[data-scene-matrix-column]");
    const targetColumn = hitElement?.closest<HTMLElement>("[data-scene-matrix-column]");
    const sourceColumnId = sourceColumn?.dataset.sceneMatrixColumn;
    const targetColumnId = targetColumn?.dataset.sceneMatrixColumn;
    const targetCueId = Number(targetCard?.dataset.sceneMatrixCueId);
    const sameColumn = sourceColumnId === targetColumnId;
    if (
      !targetColumnId ||
      (sameColumn && !targetCard) ||
      (targetCard && (!Number.isFinite(targetCueId) || targetCueId === dragPointer.cue.id))
    ) {
      setDropIndicator(null);
      setDropTargetColumnId(null);
      return;
    }
    setDropTargetColumnId(targetColumnId);
    if (!targetCard) {
      setDropIndicator(null);
      return;
    }
    const targetRect = targetCard.getBoundingClientRect();
    const targetScrollerRect = targetCard
      .closest<HTMLElement>(".sceneMatrixCards")
      ?.getBoundingClientRect();
    const targetVisibleTop = targetScrollerRect
      ? Math.max(targetRect.top, targetScrollerRect.top)
      : targetRect.top;
    const targetVisibleBottom = targetScrollerRect
      ? Math.min(targetRect.bottom, targetScrollerRect.bottom)
      : targetRect.bottom;
    const targetVisibleMidpoint = targetVisibleBottom > targetVisibleTop
      ? targetVisibleTop + (targetVisibleBottom - targetVisibleTop) / 2
      : targetRect.top + targetRect.height / 2;
    setDropIndicator({
      cueId: targetCueId,
      position: event.clientY < targetVisibleMidpoint ? "before" : "after",
    });
  };

  const moveDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    const crossedThreshold = Math.hypot(
        event.clientX - dragPointer.startClientX,
        event.clientY - dragPointer.startClientY,
      ) > SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX;
    if (!dragPointer.moved && crossedThreshold) {
      dragPointer.moved = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragCueId(dragPointer.cue.id);
      props.onBeginTimelineCueDrag(
        dragPointer.cue,
        {
          pointerId: event.pointerId,
          clientX: dragPointer.startClientX,
          clientY: dragPointer.startClientY,
        },
        "scene-matrix",
      );
    }
    if (!dragPointer.moved) return;
    event.preventDefault();
    updateDropIndicator(event);
    props.onMoveTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const finishDrag = (
    event: PointerEvent & { currentTarget: HTMLElement },
    canceled: boolean,
  ) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const moved = dragPointer.moved;
    const cueId = dragPointer.cue.id;
    dragPointer = null;
    setDragCueId(null);
    setDropTargetColumnId(null);
    setDropIndicator(null);
    if (!moved) {
      if (canceled) return;
      // WebView pointer sequences do not always synthesize a trailing click.
      // Select on pointer-up so the strip remains a reliable click target,
      // then swallow the compatibility click when the browser does emit one.
      event.preventDefault();
      suppressClickCueId = cueId;
      props.onSelectCue(cueId);
      window.setTimeout(() => {
        if (suppressClickCueId === cueId) suppressClickCueId = null;
      }, 0);
      return;
    }
    event.preventDefault();
    suppressClickCueId = cueId;
    window.setTimeout(() => {
      if (suppressClickCueId === cueId) suppressClickCueId = null;
    }, 0);
    props.onEndTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    }, moved, canceled);
  };

  const isActive = (cue: CueSummary) => cue.group_id
    ? props.activeGroupCueIds[cue.group_id] === cue.id
    : props.activeCueId === cue.id;
  const progressValue = (cue: CueSummary) => props.activeFade?.cue_id === cue.id
    ? Math.max(0, Math.min(1, props.activeFade.progress))
    : 1;

  return (
    <section
      class="sceneMatrixPanel"
      aria-label="Scene matrix grouped by scene bank"
      data-timeline-track={props.timelineTrack}
    >
      <header class="sceneMatrixSurfaceHeader">
        <div class="sceneMatrixCueListHeader sceneMatrixBankToolbarRow">
          <nav
            class="sceneMatrixCueListTabs sceneMatrixBankJumpStrip"
            aria-label="Scene bank navigation"
            data-wheel-scroll-surface="scene-cue-list-tabs"
            onWheel={handleHorizontalWheel}
          >
            <For each={cueLists()}>
              {(cueList, index) => (
                <button
                  type="button"
                  draggable={Boolean(props.onReorderCueLists)}
                  classList={{
                    active: activeBankId() === String(cueList.id),
                    bankDropTarget: bankDropTargetId() === cueList.id,
                  }}
                  data-scene-matrix-cue-list-tab={cueList.id}
                  data-scene-matrix-bank-jump={cueList.id}
                  aria-current={activeBankId() === String(cueList.id) ? "true" : undefined}
                  aria-label={`Jump to bank ${cueList.label}`}
                  style={{ "--group-identity-text": groupIdentityCss(`bank:${cueList.id}`, undefined, "text") }}
                  tabIndex={0}
                  onClick={() => {
                    selectCueList(cueList.id);
                    jumpToBank(String(cueList.id));
                  }}
                  onContextMenu={(event) => requestContextMenu(event, cueList.id)}
                  onDragStart={() => setBankDragId(cueList.id)}
                  onDragOver={(event) => {
                    if (!props.onReorderCueLists || bankDragId() === null || bankDragId() === cueList.id) return;
                    event.preventDefault();
                    setBankDropTargetId(cueList.id);
                  }}
                  onDrop={(event) => {
                    event.preventDefault();
                    const sourceId = bankDragId();
                    if (sourceId !== null) void reorderCueLists(sourceId, cueList.id);
                  }}
                  onDragEnd={() => {
                    setBankDragId(null);
                    setBankDropTargetId(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
                      event.preventDefault();
                      requestContextMenu(event, cueList.id);
                      return;
                    }
                    if (event.key === "Escape" && bankEditorMode()) {
                      event.preventDefault();
                      cancelBankEditor();
                      return;
                    }
                    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
                    event.preventDefault();
                    const list = cueLists();
                    const nextIndex = event.key === "Home"
                      ? 0
                      : event.key === "End"
                        ? list.length - 1
                        : Math.max(0, Math.min(
                          list.length - 1,
                          index() + (event.key === "ArrowLeft" ? -1 : 1),
                        ));
                    const next = list[nextIndex];
                    if (!next) return;
                    selectCueList(next.id);
                    jumpToBank(String(next.id));
                    queueMicrotask(() => document.querySelector<HTMLButtonElement>(
                      `[data-scene-matrix-cue-list-tab="${next.id}"]`,
                    )?.focus());
                  }}
                >
                  <span data-no-localize>{cueList.label}</span>
                  <small data-no-localize>{stableCues.filter((cue) => cue.cue_list_id === cueList.id).length}</small>
                </button>
              )}
            </For>
          </nav>
          <div class="sceneMatrixCueListActions">
            <Show when={bankEditorMode() === null}>
              <button
                type="button"
                data-scene-matrix-create-bank
                aria-label="Add bank"
                disabled={Boolean(props.bankAuthority.issue)}
                title={props.bankAuthority.issue
                  ? "Scene Bank editing is unavailable until the invalid Bank identity is repaired."
                  : undefined}
                onClick={beginCreateBank}
              >
                + Bank
              </button>
            </Show>
            <Show when={bankEditorMode()}>
              <form class="sceneMatrixBankEditor" onSubmit={submitBankEditor}>
                <label>
                  <span>Bank name</span>
                  <input
                    ref={(element) => { bankInputElement = element; }}
                    maxlength="64"
                    value={bankDraft()}
                    aria-invalid={Boolean(bankEditorError())}
                    aria-describedby={bankEditorError() ? "scene-matrix-bank-editor-error" : undefined}
                    onInput={(event) => setBankDraft(event.currentTarget.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Escape") {
                        event.preventDefault();
                        cancelBankEditor();
                      }
                    }}
                  />
                </label>
                <button
                  type="submit"
                  data-scene-matrix-save-bank
                  disabled={bankEditorSubmitting()}
                  aria-busy={bankEditorSubmitting() ? "true" : undefined}
                >
                  Save
                </button>
                <button
                  type="button"
                  data-scene-matrix-cancel-bank
                  disabled={bankEditorSubmitting()}
                  onClick={cancelBankEditor}
                >
                  Cancel
                </button>
                <Show when={bankEditorError()}>
                  {(error) => <span id="scene-matrix-bank-editor-error" data-scene-matrix-bank-editor-error role="alert">{error()}</span>}
                </Show>
              </form>
            </Show>
          </div>
          {props.toolbar}
        </div>
        <Show when={bankContextMenu()}>
          <div
            ref={(element) => { bankContextMenuElement = element; }}
            class="sceneMatrixBankContextMenu"
            role="menu"
            aria-label="Bank actions"
            tabindex="-1"
            style={{
              left: `${bankContextMenu()?.left ?? 0}px`,
              top: `${bankContextMenu()?.top ?? 0}px`,
            }}
            onKeyDown={(event) => {
              handleContextMenuKeyDown(event, closeContextMenu);
            }}
          >
            <button
              type="button"
              role="menuitem"
              data-scene-matrix-context-create-scene
              onClick={createContextScene}
            >
              New Scene
            </button>
            <button
              type="button"
              role="menuitem"
              data-scene-matrix-context-rename
              onClick={beginContextRename}
            >
              Rename
            </button>
            <button
              type="button"
              role="menuitem"
              data-scene-matrix-context-delete
              disabled={
                !props.onRemoveCueList ||
                cueLists().length <= 1
              }
              title={
                cueLists().length <= 1 ? "At least one bank is required." : undefined
              }
              onClick={requestContextDelete}
            >
              Delete
            </button>
            <Show when={cueLists().length <= 1}>
              <small class="sceneMatrixContextMenuHint" role="status">
                The last remaining bank cannot be deleted.
              </small>
            </Show>
          </div>
        </Show>
      </header>
      <Show when={sceneContextMenu()}>
        <div
          ref={(element) => { sceneContextMenuElement = element; }}
          class="sceneMatrixBankContextMenu sceneMatrixSceneContextMenu"
          role="menu"
          aria-label="Scene actions"
          tabindex="-1"
          style={{
            left: `${sceneContextMenu()?.left ?? 0}px`,
            top: `${sceneContextMenu()?.top ?? 0}px`,
          }}
          onKeyDown={(event) => handleContextMenuKeyDown(event, closeSceneContextMenu)}
        >
          <button
            type="button"
            role="menuitem"
            data-scene-matrix-scene-context-rename
            disabled={!props.onRenameCue}
            onClick={renameContextScene}
          >
            Rename
          </button>
          <button
            type="button"
            role="menuitem"
            data-scene-matrix-scene-context-duplicate
            disabled={!props.onDuplicateCue}
            onClick={duplicateContextScene}
          >
            Duplicate
          </button>
          <Show when={sceneContextCue()?.child_timeline}>
            <button
              type="button"
              role="menuitem"
              data-scene-matrix-scene-context-open-timeline
              onClick={openContextSceneTimeline}
            >
              Open Timeline
            </button>
          </Show>
          <button
            type="button"
            role="menuitem"
            class="danger"
            data-scene-matrix-scene-context-delete
            disabled={!props.onRemoveCue}
            onClick={requestContextSceneDelete}
          >
            Delete
          </button>
        </div>
      </Show>
      <div
        class="sceneMatrixScroller"
        ref={(element) => {
          scrollerElement = element;
        }}
        onScroll={syncVisibleBank}
        data-wheel-scroll-surface="scene-matrix-banks"
        onWheel={handleMatrixWheel}
      >
        <Show when={props.bankAuthority.issue}>
          {(issue) => (
            <p
              class="sceneMatrixAuthorityUnavailable"
              role="alert"
              data-scene-matrix-bank-authority-unavailable={issue().kind}
            >
              {bankAuthorityIssueMessage(issue())}
            </p>
          )}
        </Show>
        <div class="sceneMatrixColumns">
          <For each={columns()}>
            {(column) => {
              // The data attribute keeps the deterministic hash hue for the
              // harness. Bank identity is deliberately separate from cue
              // group_id; group colors remain cue-level metadata.
              const bankIdentity = `bank:${column.cueListId}`;
              const hue = () => groupIdentityHue(bankIdentity);
              const groupCss = (role: "fill" | "text") =>
                groupIdentityCss(bankIdentity, undefined, role);
              return (
                <section
                  class="sceneMatrixColumn"
                  style={{
                    "--group-identity": groupCss("fill"),
                    "--group-identity-text": groupCss("text"),
                  }}
                  data-scene-matrix-column={column.id}
                  data-scene-matrix-drop-target={
                    dropTargetColumnId() === column.id ? "true" : undefined
                  }
                  aria-label={`Scene matrix column ${column.label}`}
                >
                  <i class="sceneMatrixBankStrip" aria-hidden="true" />
                  <header
                    class="sceneMatrixColumnHeader"
                    classList={{ bankDropTarget: bankDropTargetId() === column.cueListId }}
                    draggable={Boolean(props.onReorderCueLists)}
                    data-scene-matrix-group-hue={hue()}
                    data-scene-matrix-column-header={column.cueListId}
                    tabIndex={0}
                    onDragStart={() => setBankDragId(column.cueListId)}
                    onDragOver={(event) => {
                      if (!props.onReorderCueLists || bankDragId() === null || bankDragId() === column.cueListId) return;
                      event.preventDefault();
                      setBankDropTargetId(column.cueListId);
                    }}
                    onDrop={(event) => {
                      event.preventDefault();
                      const sourceId = bankDragId();
                      if (sourceId !== null) void reorderCueLists(sourceId, column.cueListId);
                    }}
                    onDragEnd={() => {
                      setBankDragId(null);
                      setBankDropTargetId(null);
                    }}
                    onContextMenu={(event) => requestContextMenu(event, column.cueListId)}
                    onKeyDown={(event) => {
                      if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
                        event.preventDefault();
                        requestContextMenu(event, column.cueListId);
                      }
                    }}
                  >
                    <div class="sceneMatrixColumnHeaderIdentity">
                      <strong data-no-localize>{column.label}</strong>
                      <span>{column.cues.length}</span>
                    </div>
                    <Show when={column.cues.length > 0}>
                      <button
                        type="button"
                        class="sceneMatrixCreateScene"
                        data-scene-matrix-create-scene={column.cueListId}
                        aria-label={`Add scene to ${column.label}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          createScene(column.cueListId);
                        }}
                      >
                        + Scene
                      </button>
                    </Show>
                  </header>
                  <div
                    class="sceneMatrixCards"
                    data-scene-matrix-column-scroll
                    data-scene-matrix-column-drop-position={
                      dropTargetColumnId() === column.id && !dropIndicator()
                        ? "after"
                        : undefined
                    }
                  >
                    <Show when={column.cues.length > 0} fallback={
                      <div class="sceneMatrixEmptyAction" data-scene-matrix-empty-bank={column.cueListId}>
                        <button
                          type="button"
                          data-scene-matrix-open-cue-editor={column.cueListId}
                          data-scene-matrix-create-scene={column.cueListId}
                          onClick={() => createScene(column.cueListId)}
                        >
                          + Scene
                        </button>
                      </div>
                    }>
                      <For each={column.cues}>
                        {(cue) => {
                          const flashMode = () => authoredCueLiveModifier(cue).flash;
                          const kind = () => sceneCueKind(cue);
                          const flashRelease = (event: PointerEvent) => {
                            if (!flashMode()) return;
                            event.stopPropagation();
                            void props.onReleaseCue(cue.id);
                          };
                          return (
                            <article
                              class="sceneMatrixCard"
                              classList={{
                                active: isActive(cue),
                                selected: props.selectedCueId === cue.id,
                              }}
                              style={{
                                // Scene Matrix is bank-first: a cue's persisted
                                // identity color belongs to the editor/detail
                                // surface, while every card in this column must
                                // share the owning Bank accent after DnD,
                                // Undo/Redo, and reload.
                                "--cue-identity": groupCss("fill"),
                                "--cue-identity-text": groupCss("text"),
                              }}
                              data-scene-matrix-cue-id={cue.id}
                              data-scene-matrix-cue-list-id={cue.cue_list_id}
                              data-scene-matrix-cue-hue={hue()}
                              data-scene-matrix-active={isActive(cue) ? "true" : "false"}
                              data-scene-matrix-selected={props.selectedCueId === cue.id ? "true" : "false"}
                              data-scene-matrix-drop-position={
                                dropIndicator()?.cueId === cue.id
                                  ? dropIndicator()!.position
                                  : undefined
                              }
                              data-timeline-cue-drag-source={cue.id}
                              onContextMenu={(event) => requestSceneContextMenu(event, cue)}
                              onKeyDown={(event) => {
                                if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
                                  event.preventDefault();
                                  requestSceneContextMenu(event, cue);
                                }
                              }}
                            >
                              <button
                                type="button"
                                class="sceneMatrixTrigger"
                                classList={{ flash: flashMode() }}
                                data-scene-flash-cue={flashMode() ? cue.id : undefined}
                                {...controlMappingTargetData({
                                  action: "TriggerCue",
                                  cue_id: cue.id,
                                  label: `Cue ${cue.cue_number || cue.id} ${cue.label}`,
                                })}
                                aria-label={
                                  flashMode()
                                    ? `Flash Cue ${cue.label}`
                                    : isActive(cue)
                                      ? `Release Cue ${cue.label}`
                                      : `Trigger Cue ${cue.label}`
                                }
                                onPointerDown={(event) => {
                                  if (!flashMode()) return;
                                  // Flash pads are momentary: press activates the
                                  // scene, release always releases it, and the
                                  // press never starts a timeline drag.
                                  event.stopPropagation();
                                  event.preventDefault();
                                  event.currentTarget.setPointerCapture(event.pointerId);
                                  void props.onTriggerCue(cue.id);
                                }}
                                onPointerUp={flashRelease}
                                onPointerCancel={flashRelease}
                                onKeyDown={(event) => {
                                  if (!flashMode() || event.repeat) return;
                                  if (event.key === " " || event.key === "Enter") {
                                    event.preventDefault();
                                    void props.onTriggerCue(cue.id);
                                  }
                                }}
                                onKeyUp={(event) => {
                                  if (!flashMode()) return;
                                  if (event.key === " " || event.key === "Enter") {
                                    event.preventDefault();
                                    void props.onReleaseCue(cue.id);
                                  }
                                }}
                                onClick={(event) => {
                                  if (flashMode()) {
                                    event.preventDefault();
                                    return;
                                  }
                                  if (suppressClickCueId === cue.id) {
                                    suppressClickCueId = null;
                                    event.preventDefault();
                                    return;
                                  }
                                  if (isActive(cue)) {
                                    void props.onReleaseCue(cue.id);
                                  } else {
                                    void props.onTriggerCue(cue.id);
                                  }
                                }}
                              >
                                <span
                                  class="sceneMatrixCuePrimaryRow"
                                  data-scene-matrix-primary-row
                                >
                                  <span data-no-localize class="sceneMatrixCueNumber">{cue.cue_number || cue.id}</span>
                                  <strong
                                    data-no-localize
                                    data-scene-matrix-cue-name
                                    title={cue.label}
                                  >
                                    {cue.label}
                                  </strong>
                                </span>
                                <span
                                  class="sceneMatrixCueMetaRow"
                                  classList={{ hasSuperScene: Boolean(cue.child_timeline) }}
                                  data-scene-matrix-meta-row
                                >
                                  <span class="sceneMatrixTypeBadges">
                                    <span
                                      class={`sceneMatrixKindBadge uiMicroLabel ${kind() === "TIMELINE" ? "super" : kind().toLowerCase()}`}
                                      data-scene-matrix-kind={kind()}
                                      data-no-localize
                                    >
                                      {kind()}
                                    </span>
                                  </span>
                                  <Show when={flashMode()}>
                                    <span class="sceneMatrixFlashBadge" data-scene-flash-badge={cue.id}>
                                      FLASH
                                    </span>
                                  </Show>
                                  <Show when={(cue.recall_mode ?? "Coexist") === "ReplaceGroup"}>
                                    <span class="sceneMatrixReplaceBadge">Replace group</span>
                                  </Show>
                                  <small data-scene-matrix-time>{displayNumber(cue.fade_ms, 0)}ms</small>
                                </span>
                              </button>
                              <Show when={cue.child_timeline}>
                                <button
                                  type="button"
                                  class="sceneMatrixKindBadge superScene sceneMatrixSuperSceneAction"
                                  data-scene-matrix-super-scene={cue.id}
                                  data-no-localize
                                  title={`Open Timeline ${cue.label}`}
                                  aria-label={`Open Timeline ${cue.label}`}
                                  onPointerDown={(event) => event.stopPropagation()}
                                  onPointerMove={(event) => event.stopPropagation()}
                                  onPointerUp={(event) => event.stopPropagation()}
                                  onPointerCancel={(event) => event.stopPropagation()}
                                  onClick={(event) => {
                                    event.preventDefault();
                                    event.stopPropagation();
                                    void props.onOpenSuperScene(cue.id);
                                  }}
                                >
                                  TL
                                </button>
                              </Show>
                              <button
                                type="button"
                                class="sceneMatrixEditStrip"
                                classList={{ dragging: dragCueId() === cue.id }}
                                data-scene-matrix-edit-strip={cue.id}
                                data-scene-matrix-drag-threshold={SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX}
                                title={`Click to select Cue ${cue.label}; drag to reorder or move between banks`}
                                aria-label={`Edit scene settings for Cue ${cue.label}`}
                                aria-pressed={props.selectedCueId === cue.id}
                                onPointerDown={(event) => {
                                  event.stopPropagation();
                                  beginDrag(event, cue);
                                }}
                                onPointerMove={(event) => {
                                  event.stopPropagation();
                                  moveDrag(event);
                                }}
                                onPointerUp={(event) => {
                                  event.stopPropagation();
                                  finishDrag(event, false);
                                }}
                                onPointerCancel={(event) => {
                                  event.stopPropagation();
                                  finishDrag(event, true);
                                }}
                                onClick={(event) => {
                                  event.stopPropagation();
                                  if (suppressClickCueId === cue.id) {
                                    suppressClickCueId = null;
                                    event.preventDefault();
                                    return;
                                  }
                                  props.onSelectCue(cue.id);
                                }}
                              >
                                <span
                                  class="sceneMatrixEditStripBand"
                                  aria-hidden="true"
                                  data-no-localize
                                />
                              </button>
                              <Show when={isActive(cue)}>
                                <div class="sceneMatrixProgress" data-scene-matrix-progress={cue.id}>
                                  <span>LIVE</span>
                                  <progress
                                    max="1"
                                    value={progressValue(cue)}
                                    aria-label={`Cue ${cue.label} progress`}
                                  />
                                </div>
                              </Show>
                              <Show
                                when={
                                  isActive(cue) &&
                                  props.onSetCueLiveModifier &&
                                  props.onClearCueLiveModifier
                                }
                              >
                                <CueLiveModifierStrip
                                  cue={cue}
                                  liveStates={props.cueLiveModifiers}
                                  onSetCueLiveModifier={props.onSetCueLiveModifier!}
                                  onClearCueLiveModifier={props.onClearCueLiveModifier!}
                                />
                              </Show>
                            </article>
                          );
                        }}
                      </For>
                    </Show>
                  </div>
                </section>
              );
            }}
          </For>
        </div>
      </div>
      <Show when={bankDeleteRequest()}>
        {(request) => (
          <dialog
            ref={(dialog) => {
              bankDeleteDialogElement = dialog;
              queueMicrotask(() => {
                if (!dialog.open) dialog.showModal();
                dialog.querySelector<HTMLButtonElement>("[data-scene-matrix-delete-cancel]")?.focus();
              });
            }}
            class="protectedCloseDialog"
            data-scene-matrix-delete-dialog
            role="alertdialog"
            aria-labelledby="scene-matrix-delete-title"
            aria-describedby="scene-matrix-delete-detail"
            onCancel={(event) => {
              event.preventDefault();
              cancelContextDelete();
            }}
          >
            <section class="protectedCloseFrame">
              <header>
                <span>REMOVE BANK</span>
                <strong data-no-localize>{request().label}</strong>
              </header>
              <div class="protectedCloseBody">
                <div class="protectedCloseMark" aria-hidden="true">!</div>
                <div>
                  <h2 id="scene-matrix-delete-title" class="textBalance">Delete this bank?</h2>
                  <p id="scene-matrix-delete-detail" class="textPretty">
                    <strong data-scene-matrix-delete-scene-count>
                      {stableCues.filter((cue) => cue.cue_list_id === request().id).length} scenes
                    </strong>{" "}
                    in {request().label} will be deleted.
                  </p>
                </div>
              </div>
              <div class="protectedCloseActions">
                <button type="button" data-scene-matrix-delete-cancel onClick={cancelContextDelete}>
                  Cancel
                </button>
                <button type="button" class="danger" data-scene-matrix-delete-confirm onClick={() => void confirmContextDelete()}>
                  Delete bank
                </button>
              </div>
            </section>
          </dialog>
        )}
      </Show>
      <Show when={sceneDeleteRequest()}>
        {(request) => {
          const impact = () => props.cueRemovalImpact?.(request().id) ?? {
            linkedPlacements: 0,
            incomingJumps: 0,
          };
          return (
            <dialog
              ref={(dialog) => {
                sceneDeleteDialogElement = dialog;
                queueMicrotask(() => {
                  if (!dialog.open) dialog.showModal();
                  dialog.querySelector<HTMLButtonElement>("[data-scene-matrix-scene-delete-cancel]")?.focus();
                });
              }}
              class="protectedCloseDialog"
              data-scene-matrix-scene-delete-dialog
              role="alertdialog"
              aria-labelledby="scene-matrix-scene-delete-title"
              aria-describedby="scene-matrix-scene-delete-detail"
              onCancel={(event) => {
                event.preventDefault();
                cancelContextSceneDelete();
              }}
            >
              <section class="protectedCloseFrame">
                <header>
                  <span>REMOVE SCENE</span>
                  <strong data-no-localize>{request().label}</strong>
                </header>
                <div class="protectedCloseBody">
                  <div class="protectedCloseMark" aria-hidden="true">!</div>
                  <div>
                    <h2 id="scene-matrix-scene-delete-title" class="textBalance">Delete this scene?</h2>
                    <p id="scene-matrix-scene-delete-detail" class="textPretty">
                      <strong data-scene-matrix-scene-delete-placement-count>
                        {impact().linkedPlacements} Timeline placement{impact().linkedPlacements === 1 ? "" : "s"}
                      </strong>{" "}
                      and{" "}
                      <strong data-scene-matrix-scene-delete-jump-count>
                        {impact().incomingJumps} incoming jump{impact().incomingJumps === 1 ? "" : "s"}
                      </strong>{" "}
                      reference this scene.
                    </p>
                  </div>
                </div>
                <div class="protectedCloseActions">
                  <button
                    type="button"
                    data-scene-matrix-scene-delete-cancel
                    onClick={cancelContextSceneDelete}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    class="danger"
                    data-scene-matrix-scene-delete-confirm
                    onClick={() => void confirmContextSceneDelete()}
                  >
                    Delete scene
                  </button>
                </div>
              </section>
            </dialog>
          );
        }}
      </Show>
    </section>
  );
}
