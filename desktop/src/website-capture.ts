import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { cancelWebsiteCapture, captureWebsites, getWebsiteState } from "./api";
import { labels } from "./labels";
import { fill } from "./format";
import type { WebsiteProgress, WebsiteState } from "./types";

export function useWebsiteCapture(
  snapshotId: string | undefined,
  collecting: boolean,
) {
  const [state, setState] = useState<WebsiteState>();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<WebsiteProgress>();
  const [message, setMessage] = useState<string>();
  const [error, setError] = useState<string>();
  const currentId = useRef(snapshotId);
  currentId.current = snapshotId;
  const running = useRef(false);
  const generation = useRef(0);
  const load = useCallback(async (id: string) => {
    try {
      const next = await getWebsiteState(id);
      if (currentId.current === id) setState(next);
    } catch (caught) {
      if (currentId.current === id) setError(String(caught));
    }
  }, []);

  useEffect(() => {
    setState(undefined);
    setError(undefined);
    setMessage(undefined);
    if (snapshotId && !collecting) void load(snapshotId);
  }, [snapshotId, collecting, load]);

  async function capture(urls: string[] = [], retryOnly = false) {
    if (!snapshotId || collecting || running.current) return;
    running.current = true;
    const batch = ++generation.current;
    setBusy(true);
    setError(undefined);
    setMessage(undefined);
    try {
      const result = await captureWebsites(
        { snapshotId, urls, retryOnly },
        (next) => {
          if (running.current && generation.current === batch)
            setProgress(next.url ? next : undefined);
        },
      );
      const words = labels().common.websites;
      setMessage(
        result.cancelled
          ? words.cancelled
          : fill(words.complete, {
              captured: result.captured,
              failed: result.failed,
              skipped: result.skipped,
            }),
      );
      if (result.error) setError(result.error);
    } catch (caught) {
      setError(String(caught));
    } finally {
      running.current = false;
      setBusy(false);
      setProgress(undefined);
      if (currentId.current) await load(currentId.current);
    }
  }
  async function cancel() {
    try {
      await cancelWebsiteCapture();
    } catch (caught) {
      setError(String(caught));
    }
  }
  return {
    state,
    busy,
    blocked: busy || collecting,
    progress,
    message,
    error,
    capture,
    cancel,
  };
}

export const WebsiteContext = createContext<ReturnType<
  typeof useWebsiteCapture
> | null>(null);
export const useWebsites = () => useContext(WebsiteContext);
