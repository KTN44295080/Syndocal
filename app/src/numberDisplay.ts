export const displayNumber = (
  value: number | null | undefined,
  maximumFractionDigits: number,
) => {
  if (value === null || value === undefined || !Number.isFinite(value)) return "";
  const scale = 10 ** maximumFractionDigits;
  const rounded = Math.round((value + Number.EPSILON) * scale) / scale;
  return String(Object.is(rounded, -0) ? 0 : rounded);
};
