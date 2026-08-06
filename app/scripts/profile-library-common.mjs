// Shared conversion primitives for the offline fixture-profile bundles.

export const normalizeProfileIdentity = (value) => value
  .normalize("NFKD")
  .replace(/\p{Diacritic}/gu, "")
  .toLocaleLowerCase()
  .replace(/[^a-z0-9]+/g, "");

export const fixtureIdentity = (manufacturer, fixture) =>
  `${normalizeProfileIdentity(manufacturer)}\u0000${normalizeProfileIdentity(fixture)}`;

export const encodedAttributeFootprint = (attributes) => attributes.reduce(
  (total, attribute) => total + (attribute.endsWith(":16") ? 2 : 1),
  0,
);

/**
 * Convert one slot per DMX byte into Syndocal's unique attribute layout.
 * A slot may consume two adjacent bytes when a source explicitly identifies a
 * coarse/fine pair. Unknown channels stay addressable through stable generic
 * names, so conversion never changes the source mode footprint.
 */
export const encodeAttributeSlots = (slots) => {
  const seen = new Map();
  let offset = 1;
  return slots.map((slot) => {
    const count = (seen.get(slot.attribute) ?? 0) + 1;
    seen.set(slot.attribute, count);
    const attribute = count === 1 ? slot.attribute : `${slot.attribute}_${count}`;
    const encoded = `${attribute}@${offset}:${slot.bits}`;
    offset += slot.bits === 16 ? 2 : 1;
    return encoded;
  });
};
