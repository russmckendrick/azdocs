// Copies the assets the app actually references into desktop/public/, which
// Vite serves and bundles. The whole data/ directory used to be the public
// root, so every build shipped 700+ stock icons and the report fonts the
// webview never loads. The referenced set is read from the source, so a new
// icon path in azure-icons.ts or mock-data.ts is picked up on the next run.
//
//     node scripts/sync-assets.mjs
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dataDir = resolve(desktopDir, "../data");
const publicDir = resolve(desktopDir, "public");

const sources = ["src/azure-icons.ts", "src/mock-data.ts"].map((file) =>
  readFileSync(resolve(desktopDir, file), "utf8"),
);
const icons = new Set();
for (const source of sources) {
  for (const match of source.matchAll(/["'`]\/icons\/([^"'`]+\.svg)["'`]/g)) icons.add(match[1]);
}

rmSync(publicDir, { recursive: true, force: true });
let copied = 0;
for (const icon of icons) {
  const from = resolve(dataDir, "icons", icon);
  if (!existsSync(from)) {
    console.error(`referenced icon is missing from data/icons: ${icon}`);
    process.exit(1);
  }
  const to = resolve(publicDir, "icons", icon);
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
  copied += 1;
}
// The interface face is Inter, bundled from @fontsource-variable/inter; only
// the monospace fallback the stylesheet declares still comes from data/fonts.
const fontsDir = resolve(dataDir, "fonts");
for (const font of readdirSync(fontsDir).filter((name) => /^IBMPlexMono-.*\.ttf$/.test(name))) {
  const to = resolve(publicDir, "fonts", font);
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(resolve(fontsDir, font), to);
  copied += 1;
}
console.log(`synced ${copied} assets into desktop/public`);
