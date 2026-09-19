const KIND_CLASSES = new Set(["network", "structure", "data", "identity", "monitoring"]);

/// Every colour the canvas paints comes from the design-language tokens in
/// styles.css, resolved at graph build time so both themes use one palette.
export function cssToken(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

export function kindClassColor(kindClass: string) {
  return cssToken(KIND_CLASSES.has(kindClass) ? `--kind-${kindClass}` : "--cat-other", "#5a6470");
}
