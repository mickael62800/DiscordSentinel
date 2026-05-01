/// <reference types="vitest" />
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { compression } from "vite-plugin-compression2";
import { visualizer } from "rollup-plugin-visualizer";
import { fileURLToPath, URL } from "node:url";

// App web standalone : la couche Vue parle directement a l'API Axum via fetch.
// Aucune dependance Tauri : tout passe par src/api/*.
//
// Build : `npm run build`
// Analyse bundle : `npm run build:analyze` -> genere dist/stats.html
export default defineConfig({
  plugins: [
    vue(),
    // Pre-compression a la build : nginx les sert directement (gzip_static on)
    // sans compresser a chaque requete. Threshold 1 KB : pas la peine pour
    // les petits fichiers.
    compression({ algorithm: "gzip", threshold: 1024 }),
    // Visualizer du bundle : actif uniquement avec ANALYZE=1.
    // Ouvre dist/stats.html apres build pour voir treemap des chunks.
    process.env.ANALYZE === "1" && visualizer({
      filename: "dist/stats.html",
      gzipSize: true,
      brotliSize: true,
      template: "treemap",
    }),
  ].filter(Boolean) as any,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    host: true,
    port: 5180,
    strictPort: false,
  },
  build: {
    // Code splitting : separer Vue/router/Pinia de Chart.js (~190 KB) pour
    // que les pages sans graphes ne paient pas le cout de chart.js.
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-vue": ["vue", "vue-router", "pinia"],
          "vendor-charts": ["chart.js", "vue-chartjs"],
        },
      },
    },
    // Desactive le polyfill modulePreload qui injecte du JS inline dans
    // index.html. Les navigateurs modernes supportent <link rel="modulepreload">
    // nativement (Chrome 66+, Firefox 115+, Safari 15+). Sans polyfill, plus
    // aucun inline script -> on peut retirer 'unsafe-inline' du CSP nginx.
    modulePreload: { polyfill: false },
    // Genere un manifest pour debug du splitting (optionnel mais utile).
    reportCompressedSize: true,
    chunkSizeWarningLimit: 500,
  },
  test: {
    environment: "happy-dom",
    globals: true,
    include: ["src/**/*.{test,spec}.ts"],
  },
});
