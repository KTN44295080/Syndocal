const wheelDeltaInPixels = (event: WheelEvent, scroller: HTMLElement) => {
  const scale = event.deltaMode === WheelEvent.DOM_DELTA_LINE
    ? 16
    : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
      ? Math.max(1, scroller.clientWidth)
      : 1;
  const dominantDelta = Math.abs(event.deltaX) > Math.abs(event.deltaY)
    ? event.deltaX
    : event.deltaY;
  return dominantDelta * scale;
};

export const elementHasHorizontalOverflow = (element: HTMLElement) =>
  element.scrollWidth > element.clientWidth + 1;

export const handleHorizontalWheel = (
  event: WheelEvent & { currentTarget: HTMLElement },
) => {
  if (event.ctrlKey || !elementHasHorizontalOverflow(event.currentTarget)) {
    return false;
  }
  const delta = wheelDeltaInPixels(event, event.currentTarget);
  if (!Number.isFinite(delta) || Math.abs(delta) < 0.01) {
    return false;
  }
  const before = event.currentTarget.scrollLeft;
  event.currentTarget.scrollLeft += delta;
  if (Math.abs(event.currentTarget.scrollLeft - before) < 0.01) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
};

