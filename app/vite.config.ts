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
      preserveEntrySignatures: "exports-only",
      output: {
        // Preserve source-module boundaries instead of combining the entire
        // application controller into one bootstrap chunk. Large offline
        // profile data is emitted as JSON assets by bundledLibrary.ts.
        preserveModules: true,
        preserveModulesRoot: "src",
        entryFileNames: (chunk) => {
          const fileName = chunk.name.split(/[\\/]/).pop() ?? "module";
          return `assets/${fileName}-[hash].js`;
        },
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
