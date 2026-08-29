import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = resolve(desktopDir, "../docs/marks/assets/azdocs-app-icon-light.svg");
const iconDir = resolve(desktopDir, "src-tauri/icons");
const stamp = resolve(iconDir, ".azdocs-app-icon-light.sha256");
const requiredOutputs = [
  "32x32.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.icns",
  "icon.ico",
];

const hash = createHash("sha256").update(readFileSync(source)).digest("hex");
const recordedHash = existsSync(stamp) ? readFileSync(stamp, "utf8").trim() : "";
const outputsExist = requiredOutputs.every((name) => existsSync(resolve(iconDir, name)));
const force = process.argv.includes("--force");

if (!force && recordedHash === hash && outputsExist) {
  console.log("Native icons already match docs/marks/assets/azdocs-app-icon-light.svg");
  process.exit(0);
}

const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const result = spawnSync(npm, ["run", "icons:generate"], {
  cwd: desktopDir,
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

writeFileSync(stamp, `${hash}\n`);
