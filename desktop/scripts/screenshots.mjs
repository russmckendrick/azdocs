// Captures the browser preview (`pnpm run dev`) for the docs: every main
// view in light and dark at 1440×900, written to docs/usage/assets/. The
// preview shows the illustrative estate, so the pictures are reproducible
// and never contain a real tenant. Needs a local Chrome.
//
//     pnpm run dev                 # in one terminal
//     pnpm run screenshots         # in another
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = resolve(desktopDir, "../docs/usage/assets");
const url = process.env.AZDOCS_PREVIEW_URL ?? "http://127.0.0.1:1420/";
const chrome =
  process.env.CHROME_PATH ??
  {
    darwin: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    linux: "/usr/bin/google-chrome",
    win32: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  }[process.platform];

// aria-label prefixes of the side-nav rows, in the order the docs show them,
// and for a view that is a tab inside a section, the tab's text.
const VIEWS = [
  ["overview", "Overview"],
  ["estate", "Estate"],
  ["map", "Explore"],
  ["regions", "Regions"],
  ["query-results", "Estate", "Query results"],
  ["findings", "Findings"],
  ["governance", "Governance"],
  ["changes", "Changes"],
];

mkdirSync(outDir, { recursive: true });
const browser = await puppeteer.launch({ executablePath: chrome, headless: true });
try {
  for (const theme of ["light", "dark"]) {
    const page = await browser.newPage();
    await page.setViewport({ width: 1440, height: 900, deviceScaleFactor: 1 });
    await page.emulateMediaFeatures([
      { name: "prefers-color-scheme", value: theme },
      // Number tickers and camera moves settle instantly.
      { name: "prefers-reduced-motion", value: "reduce" },
    ]);
    await page.goto(url, { waitUntil: "networkidle0" });
    await page.waitForSelector('button[aria-label^="Estate"]');
    await page.waitForSelector(".loading-workspace", { hidden: true });
    for (const [slug, label, tab] of VIEWS) {
      await page.click(`nav.side-nav button[aria-label^="${label}"]`);
      if (tab) await page.click(`nav.view-tabs ::-p-text(${tab})`);
      // The map lays out on a canvas; give it a moment after the route.
      await new Promise((done) => setTimeout(done, slug === "map" ? 2500 : 600));
      const file = resolve(outDir, `desktop-${slug}-${theme}.png`);
      await page.screenshot({ path: file });
      console.log(`wrote ${file}`);
    }
    await page.close();
  }
} finally {
  await browser.close();
}
