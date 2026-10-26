import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  build: { outDir: "dist", emptyOutDir: true, manifest: true },
  server: {
    proxy: Object.fromEntries(
      ["/api", "/d"].map((path) => [
        path,
        {
          target: "http://localhost:8080",
          changeOrigin: true,
          // The Rust server checks Origin against its configured public URL.
          headers: { origin: "http://localhost:8080" },
        },
      ]),
    ),
  },
});
