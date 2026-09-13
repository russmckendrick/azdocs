import type {
  CollectionEvent,
  ConnectionCheck,
  CollectionQueryProgress,
  CollectionStage,
  CollectResult,
  WebsiteProgress,
} from "./types";

export type CollectionFeedback = {
  startedAt: number;
  endedAt?: number;
  stage: CollectionStage | "complete" | "failed";
  queries?: CollectionQueryProgress;
  screenshots?: WebsiteProgress;
  result?: CollectResult;
  permissions?: ConnectionCheck;
};
export type FeedbackAction =
  | { type: "reset" }
  | { type: "start"; at: number }
  | { type: "event"; event: CollectionEvent }
  | { type: "finish"; at: number; result: CollectResult }
  | { type: "fail"; at: number };

export function collectionFeedback(
  state: CollectionFeedback | undefined,
  action: FeedbackAction,
): CollectionFeedback | undefined {
  if (action.type === "reset") return undefined;
  if (action.type === "start")
    return { startedAt: action.at, stage: "inventory" };
  if (!state) return state;
  if (action.type === "finish")
    return {
      ...state,
      stage: "complete",
      endedAt: action.at,
      result: action.result,
    };
  if (action.type === "fail")
    return { ...state, stage: "failed", endedAt: action.at };
  if (state.endedAt !== undefined) return state;
  const event = action.event;
  if (event.event === "permissions")
    return { ...state, permissions: event.data.check };
  if (event.event === "stage") return { ...state, stage: event.data.stage };
  if (event.event === "queries")
    return { ...state, queries: event.data.progress };
  if (event.event === "screenshots")
    return { ...state, stage: "capture", screenshots: event.data.progress };
  return state;
}

export function elapsedTime(start: number, end: number) {
  const seconds = Math.max(0, Math.floor((end - start) / 1000));
  return `${Math.floor(seconds / 60)
    .toString()
    .padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
}
