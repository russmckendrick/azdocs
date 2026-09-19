// One-shot migration of styles.css from px to rem for type and spacing, at
// the 13px base the design language was drawn at. Checked in so the
// conversion is reproducible and reviewable, not so it runs again: borders,
// radii, fixed sizes, offsets, media queries and the --graph-* tokens stay in
// px on purpose, because scaling text must not scale hairlines or the canvas.
//
//     node scripts/px-to-rem.mjs            # rewrites src/styles.css
//     node scripts/px-to-rem.mjs --dry-run  # prints the changed lines
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const BASE = 13;
const target = resolve(dirname(fileURLToPath(import.meta.url)), "../src/styles.css");
const dryRun = process.argv.includes("--dry-run");

const PROPERTIES =
  /(?<![\w-])(font-size|font|line-height|padding(?:-(?:top|right|bottom|left|block|inline)(?:-(?:start|end))?)?|margin(?:-(?:top|right|bottom|left|block|inline)(?:-(?:start|end))?)?|gap|row-gap|column-gap)\s*:\s*([^;{}]+)/g;

function rem(px) {
  const value = Number(px) / BASE;
  return `${Number(value.toFixed(4))}rem`;
}

function convertValue(value) {
  // `0px` stays `0`; anything else in px becomes rem. Values inside calc()
  // or var() fallbacks are converted the same way — they are still lengths.
  return value.replace(/(-?\d*\.?\d+)px\b/g, (match, number) =>
    Number(number) === 0 ? "0" : rem(number),
  );
}

const source = readFileSync(target, "utf8");
const changed = [];
const output = source
  .split("\n")
  .map((line, index) => {
    // Custom properties keep their units: the graph tokens and chrome
    // heights are read by JavaScript and by fixed layout rules.
    if (/^\s*--[\w-]+\s*:/.test(line)) return line;
    const next = line.replace(PROPERTIES, (whole, property, value) => {
      const converted = convertValue(value);
      return converted === value ? whole : `${property}: ${converted}`;
    });
    if (next !== line) changed.push(`${index + 1}: ${line.trim()}  ->  ${next.trim()}`);
    return next;
  })
  .join("\n");

if (dryRun) {
  console.log(changed.join("\n"));
  console.log(`\n${changed.length} lines would change`);
} else {
  writeFileSync(target, output);
  console.log(`${changed.length} lines converted to rem at ${BASE}px`);
}
