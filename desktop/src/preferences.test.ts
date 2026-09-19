import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clearPreference, readPreference, tenantKey, writePreference } from "./preferences";

function fakeStorage(store: Map<string, string> = new Map()) {
  return {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => void store.set(key, value),
    removeItem: (key: string) => void store.delete(key),
    store,
  };
}

describe("preferences", () => {
  beforeEach(() => {
    vi.stubGlobal("window", { localStorage: fakeStorage() });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });
  it("round-trips a value under the azdocs prefix", () => {
    writePreference("theme", "dark");
    expect(readPreference("theme", "system")).toBe("dark");
    expect(window.localStorage.getItem("azdocs-theme")).toBe('"dark"');
    clearPreference("theme");
    expect(readPreference("theme", "system")).toBe("system");
  });
  it("falls back when storage throws or holds junk", () => {
    vi.stubGlobal("window", {
      localStorage: {
        getItem: () => {
          throw new Error("blocked");
        },
        setItem: () => {
          throw new Error("blocked");
        },
      },
    });
    expect(readPreference("theme", "system")).toBe("system");
    expect(() => writePreference("theme", "dark")).not.toThrow();
    vi.stubGlobal("window", { localStorage: fakeStorage(new Map([["azdocs-x", "{not json"]])) });
    expect(readPreference("x", 1)).toBe(1);
  });
  it("scopes estate state by tenant", () => {
    expect(tenantKey("t1", "explorer")).toBe("explorer:t1");
    expect(tenantKey(undefined, "explorer")).toBe("explorer:no-tenant");
  });
});
