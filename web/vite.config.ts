import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import { readFileSync } from "node:fs";

const manifest = readFileSync(
  new URL("../Cargo.toml", import.meta.url),
  "utf8",
);
const workspacePackage = manifest
  .split(/^\[workspace\.package\]\s*$/m)[1]
  ?.split(/^\[/m)[0];
const version = workspacePackage?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version)
  throw new Error("Missing workspace.package.version in Cargo.toml");

export default defineConfig({
  define: { __BINBIN_VERSION__: JSON.stringify(version) },
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
