import type { ControlLearnMode, ControlMappingTarget } from './controlMappingLearn';

export function installControlLearnCapture(options: {
  mode: () => ControlLearnMode | null;
  busy: () => boolean;
  setBusy: (busy: boolean) => void;
  targets: (element: HTMLElement) => ControlMappingTarget[];
  select: (element: HTMLElement) => void;
  learn: (mode: ControlLearnMode, targets: ControlMappingTarget[]) => Promise<unknown>;
  message: (message: string) => void;
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
    options.select(element);
    options.setBusy(true);
    void Promise.resolve().then(() => options.learn(mode, targets))
      .catch(error => options.message(String(error)))
      .finally(() => { if (!disposed) options.setBusy(false); });
  };
  events.addEventListener('pointerdown', capture, true);
  events.addEventListener('click', capture, true);
  events.addEventListener('keydown', capture, true);
  return () => {
    disposed = true;
    events.removeEventListener('pointerdown', capture, true);
    events.removeEventListener('click', capture, true);
    events.removeEventListener('keydown', capture, true);
  };
}
