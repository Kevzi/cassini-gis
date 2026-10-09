import { defineConfig } from "vite";

// Configuration alignée sur Tauri : port fixe, pas d'effacement d'écran, et
// aucune ressource distante (tout est groupé dans dist/).
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
    assetsInlineLimit: 0,
  },
});
