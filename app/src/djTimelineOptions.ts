export interface DjTimelineOption {
  id: number;
  label: string;
}

interface AuthoredTimelineOptionSource {
  id?: number;
  label?: string;
}

export function retainDjTimelineOptions(
  previous: DjTimelineOption[],
  timelines: AuthoredTimelineOptionSource[],
): DjTimelineOption[] {
  const next = timelines
    .filter((timeline) => Number.isSafeInteger(timeline.id) && (timeline.id ?? 0) > 0)
    .map((timeline) => ({
      id: timeline.id!,
      label: timeline.label?.trim() || `Timeline ${timeline.id}`,
    }));

  const unchanged = previous.length === next.length
    && previous.every((option, index) => (
      option.id === next[index].id && option.label === next[index].label
    ));
  return unchanged ? previous : next;
}
