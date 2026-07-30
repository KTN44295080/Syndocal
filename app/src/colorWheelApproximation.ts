import { normalizeHexColor, rgbToHsv } from "./numericHelpers";

interface ColorCandidate {
  color: string;
}

const hsvForHexColor = (hexColor: string) => {
  const normalized = normalizeHexColor(hexColor);
  if (!normalized) {
    return null;
  }
  return rgbToHsv(
    Number.parseInt(normalized.slice(1, 3), 16),
    Number.parseInt(normalized.slice(3, 5), 16),
    Number.parseInt(normalized.slice(5, 7), 16),
  );
};

// Perceptual-ish cylindrical HSV/HSL distance:
// C = V*S, L = V*(1-S/2), X = C*cos(H), Y = C*sin(H)
// d = sqrt((dX)^2 + (dY)^2 + 1.5*(dL)^2).
// Hue therefore collapses for achromatic colors while lightness differences
// receive extra weight.
export const perceptualHsvDistance = (leftColor: string, rightColor: string) => {
  const left = hsvForHexColor(leftColor);
  const right = hsvForHexColor(rightColor);
  if (!left || !right) {
    return Number.POSITIVE_INFINITY;
  }
  const cylindrical = (color: typeof left) => {
    const hueRadians = (color.hue * Math.PI) / 180;
    const chroma = color.value * color.saturation;
    return {
      x: chroma * Math.cos(hueRadians),
      y: chroma * Math.sin(hueRadians),
      lightness: color.value * (1 - color.saturation / 2),
    };
  };
  const leftPoint = cylindrical(left);
  const rightPoint = cylindrical(right);
  const deltaX = leftPoint.x - rightPoint.x;
  const deltaY = leftPoint.y - rightPoint.y;
  const deltaLightness = leftPoint.lightness - rightPoint.lightness;
  return Math.sqrt(deltaX * deltaX + deltaY * deltaY + 1.5 * deltaLightness * deltaLightness);
};

export const nearestColorWheelEntry = <T extends ColorCandidate>(entries: readonly T[], targetColor: string) => {
  let nearest: T | undefined;
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (const entry of entries) {
    const distance = perceptualHsvDistance(targetColor, entry.color);
    if (distance < nearestDistance) {
      nearest = entry;
      nearestDistance = distance;
    }
  }
  return nearest;
};
