export interface VirtualListRangeInput {
  itemCount: number;
  itemHeight: number;
  scrollTop: number;
  viewportHeight: number;
  overscan?: number;
}

export interface VirtualListRange {
  startIndex: number;
  endIndex: number;
  offsetPx: number;
  totalHeightPx: number;
}

const finiteNonNegative = (value: number) => Number.isFinite(value) ? Math.max(0, value) : 0;

/**
 * Calculates an exclusive virtual-list window without depending on browser APIs.
 * Keeping this pure makes the large-show rendering boundary deterministic and testable.
 */
export const virtualListRange = (input: VirtualListRangeInput): VirtualListRange => {
  const itemCount = Math.max(0, Math.floor(finiteNonNegative(input.itemCount)));
  const itemHeight = Math.max(1, finiteNonNegative(input.itemHeight));
  const viewportHeight = finiteNonNegative(input.viewportHeight);
  const overscan = Math.max(0, Math.floor(finiteNonNegative(input.overscan ?? 0)));
  const totalHeightPx = itemCount * itemHeight;
  const maxScrollTop = Math.max(0, totalHeightPx - viewportHeight);
  const scrollTop = Math.min(finiteNonNegative(input.scrollTop), maxScrollTop);
  const firstVisible = Math.floor(scrollTop / itemHeight);
  const visibleCount = Math.max(1, Math.ceil(viewportHeight / itemHeight));
  const startIndex = Math.max(0, firstVisible - overscan);
  const endIndex = Math.min(itemCount, firstVisible + visibleCount + overscan);

  return {
    startIndex,
    endIndex,
    offsetPx: startIndex * itemHeight,
    totalHeightPx,
  };
};
