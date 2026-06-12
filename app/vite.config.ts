import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [
    solid({
      babel: {
        compact: false,
      },
    }),
  ],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          const normalizedId = id.replace(/\\/g, "/");
          if (
            normalizedId.includes("/src/components/ProjectorMapEditor") ||
            normalizedId.includes("/src/components/VideoColorFxEditor") ||
            normalizedId.includes("/src/components/VideoCropEditor") ||
            normalizedId.includes("/src/components/VideoPlaybackTimeline") ||
            normalizedId.includes("/src/components/VideoTransformEditor")
          ) {
            return "mapping-editors";
          }
          if (
            normalizedId.includes("/src/components/ChannelFunctionPanel") ||
            normalizedId.includes("/src/components/CategoryQuickPanel") ||
            normalizedId.includes("/src/components/ColorControlPanel") ||
            normalizedId.includes("/src/components/DimmerControlPanel") ||
            normalizedId.includes("/src/components/OpticsControlPanel") ||
            normalizedId.includes("/src/components/PositionControlPanel") ||
            normalizedId.includes("/src/components/WheelSlotPanel")
          ) {
            return "control-panels";
          }
          if (
            normalizedId.includes("/src/components/GroupChip") ||
            normalizedId.includes("/src/components/WorkspaceChrome")
          ) {
            return "workspace-chrome";
          }
          if (!normalizedId.includes("node_modules")) {
            return undefined;
          }
          if (normalizedId.includes("solid-js")) {
            return "solid";
          }
          if (normalizedId.includes("@tauri-apps")) {
            return "tauri";
          }
          return "vendor";
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
