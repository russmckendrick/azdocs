// Native delegates also deny permissions. These shims avoid prompting APIs in
// subframes and return ordinary denial results to applications requesting them.
(() => {
  const define = (object, key, value) => {
    try { Object.defineProperty(object, key, { value, configurable: false }); } catch {}
  };
  define(window, "alert", () => {});
  define(window, "confirm", () => false);
  define(window, "prompt", () => null);
  if (navigator.geolocation) {
    const denied = (_success, failure) => { if (failure) queueMicrotask(() => failure({ code: 1, message: "Permission denied" })); return 0; };
    define(navigator.geolocation, "getCurrentPosition", denied);
    define(navigator.geolocation, "watchPosition", denied);
  }
  if (window.Notification) define(Notification, "requestPermission", async () => "denied");
})();
