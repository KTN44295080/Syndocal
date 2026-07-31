export const stageOverlayHandleScreenSizePx = 22;
export const stageOverlayHandleMinimumHitSizePx = 16;
export const stageOverlayHandleGapScreenPx = 6;

export const stageWorldPerCssPixel = (
  viewBox: { width: number; height: number },
  viewportPixelSize: { width: number; height: number } | null,
) => {
  if (
    viewportPixelSize
    && Number.isFinite(viewportPixelSize.width)
    && Number.isFinite(viewportPixelSize.height)
    && viewportPixelSize.width > 0
    && viewportPixelSize.height > 0
  ) {
    return Math.max(
      viewBox.width / viewportPixelSize.width,
      viewBox.height / viewportPixelSize.height,
    );
  }
  return Math.max(viewBox.width, viewBox.height) / 100;
};

export const stageOverlayHandleWorldSize = (worldPerCssPixel: number) =>
  stageOverlayHandleScreenSizePx * worldPerCssPixel;

export const stageOverlayHandleWorldRadius = (worldPerCssPixel: number) =>
  stageOverlayHandleWorldSize(worldPerCssPixel) / 2;

export const stageOverlayHandleWorldOffsetFromEdge = (worldPerCssPixel: number) =>
  (stageOverlayHandleScreenSizePx / 2 + stageOverlayHandleGapScreenPx) * worldPerCssPixel;

export const stageFixtureYawHandlePoint = (
  fixture: { x: number; z: number; height: number; yaw: number },
  worldPerCssPixel: number,
) => {
  const angle = ((fixture.yaw - 90) * Math.PI) / 180;
  const distance = fixture.height / 2 + stageOverlayHandleWorldOffsetFromEdge(worldPerCssPixel);
  return {
    x: fixture.x + Math.cos(angle) * distance,
    z: fixture.z + Math.sin(angle) * distance,
  };
};
