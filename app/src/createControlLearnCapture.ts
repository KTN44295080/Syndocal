import type { ControlLearnMode, ControlMappingTarget } from './controlMappingLearn';

/** A control picked in Learn mode for source assignment (right-click or the
 * keyboard context-menu key). `learn` starts the ordinary Learn for it. */
export interface ControlAssignRequest {
  mode: ControlLearnMode;
  element: HTMLElement;
  targets: ControlMappingTarget[];
  x: number;
  y: number;
  learn: () => void;
}

export function installControlLearnCapture(options: {
  mode: () => ControlLearnMode | null;
  busy: () => boolean;
  setBusy: (busy: boolean) => void;
  targets: (element: HTMLElement) => ControlMappingTarget[];
  select: (element: HTMLElement) => void;
  learn: (mode: ControlLearnMode, targets: ControlMappingTarget[]) => Promise<unknown>;
  message: (message: string) => void;
  /** Offers the source choice for a control. It returns whether it took the
   * request; when it did not (or is absent) the context menu is left alone. */
  assign?: (request: ControlAssignRequest) => boolean;
}, events: Document = document): () => void {
  let pointerTarget: HTMLElement | null = null;
  let disposed = false;
  const capture = (event: MouseEvent | KeyboardEvent) => {
    const mode = options.mode();
    if (!mode || !(event.target instanceof Element)) return;
    if (event.target.closest('[data-control-learn-toggle]')) return;
    const element = event.target.closest<HTMLElement>('[data-control-map-target]');
    if (!element) return;
    if (event.type === 'pointerdown' && (event as MouseEvent).button !== 0) return;
    if (event.type === 'keydown' && (!(element instanceof HTMLInputElement) || element.type !== 'range'
      || !['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown'].includes((event as KeyboardEvent).key))) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.type === 'click' && (event as MouseEvent).detail > 0 && pointerTarget === element) {
      pointerTarget = null;
      return;
    }
    if (event.type === 'pointerdown') pointerTarget = element;
    if (options.busy()) return;
    const targets = options.targets(element);
    if (targets.length === 0) { options.message('This control cannot be mapped with the current control action model.'); return; }
    begin(mode, element, targets);
  };
  const begin = (mode: ControlLearnMode, element: HTMLElement, targets: ControlMappingTarget[]) => {
    if (options.busy()) return;
    options.select(element);
    options.setBusy(true);
    void Promise.resolve().then(() => options.learn(mode, targets))
      .catch(error => options.message(String(error)))
      .finally(() => { if (!disposed) options.setBusy(false); });
  };
  const contextMenu = (event: MouseEvent) => {
    const mode = options.mode();
    if (!mode || !options.assign || !(event.target instanceof Element)) return;
    const element = event.target.closest<HTMLElement>('[data-control-map-target]');
    if (!element) return;
    if (options.busy()) return;
    const targets = options.targets(element);
    if (targets.length === 0) return;
    // A keyboard-opened menu reports no pointer position; anchor it to the control.
    const bounds = element.getBoundingClientRect();
    const fromKeyboard = event.clientX === 0 && event.clientY === 0;
    const offered = options.assign({
      mode, element, targets,
      x: fromKeyboard ? bounds.left : event.clientX,
      y: fromKeyboard ? bounds.bottom : event.clientY,
      learn: () => begin(mode, element, targets),
    });
    if (!offered) return;
    // In Learn the control's own context menu (clip options, ...) gives way to the source choice.
    event.preventDefault();
    event.stopImmediatePropagation();
    options.select(element);
  };
  events.addEventListener('pointerdown', capture, true);
  events.addEventListener('click', capture, true);
  events.addEventListener('keydown', capture, true);
  events.addEventListener('contextmenu', contextMenu, true);
  return () => {
    disposed = true;
    events.removeEventListener('contextmenu', contextMenu, true);
    events.removeEventListener('pointerdown', capture, true);
    events.removeEventListener('click', capture, true);
    events.removeEventListener('keydown', capture, true);
  };
}
