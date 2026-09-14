import { useEffect, useRef } from "react";

import { checkExternalChanges } from "../backend";
import type { ExternalChangeStatus } from "../types";

export const EXTERNAL_CHANGE_POLL_INTERVAL_MS = 2_000;
export const EXTERNAL_CHANGE_DEBOUNCE_MS = 250;

export type ExternalChangeChecker = () => Promise<ExternalChangeStatus>;

export type UseExternalChangesOptions = {
  enabled: boolean;
  onChange: (status: ExternalChangeStatus) => void | Promise<void>;
  checker?: ExternalChangeChecker;
  pollIntervalMs?: number;
  debounceMs?: number;
};

/**
 * Watch the native read-only probe while a data-bearing view is visible.
 *
 * The hook deliberately keeps the checker and refresh callback in refs so a
 * render caused by a draft edit cannot restart the interval or lose an
 * in-flight result.  A generation token invalidates responses that resolve
 * after the view has been hidden/unmounted.
 */
export function useExternalChanges({
  enabled,
  onChange,
  checker = checkExternalChanges,
  pollIntervalMs = EXTERNAL_CHANGE_POLL_INTERVAL_MS,
  debounceMs = EXTERNAL_CHANGE_DEBOUNCE_MS,
}: UseExternalChangesOptions) {
  const onChangeRef = useRef(onChange);
  const checkerRef = useRef(checker);
  const pendingChangeRef = useRef<ExternalChangeStatus | null>(null);
  const pendingReplayRef = useRef<(() => void) | null>(null);

  onChangeRef.current = onChange;
  checkerRef.current = checker;

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    let generation = 0;
    let checkInFlight: Promise<void> | null = null;
    let refreshInFlight: Promise<void> | null = null;
    let pendingStatus: ExternalChangeStatus | null = null;
    let debounceTimer: number | null = null;

    const flush = () => {
      debounceTimer = null;
      const nextStatus = pendingStatus;
      pendingStatus = null;
      if (!nextStatus || refreshInFlight) {
        if (nextStatus) {
          pendingStatus = nextStatus;
        }
        return;
      }

      const refresh = Promise.resolve(onChangeRef.current(nextStatus))
        .catch(() => undefined)
        .finally(() => {
          refreshInFlight = null;
          if (pendingStatus && debounceTimer === null) {
            debounceTimer = window.setTimeout(flush, debounceMs);
          }
        });
      refreshInFlight = refresh;
    };

    const scheduleRefresh = (status: ExternalChangeStatus) => {
      pendingStatus = status;
      if (debounceTimer === null && !refreshInFlight) {
        debounceTimer = window.setTimeout(flush, debounceMs);
      }
    };

    const replayPending = () => {
      if (document.visibilityState === "hidden") {
        return;
      }
      const pendingChange = pendingChangeRef.current;
      pendingChangeRef.current = null;
      if (pendingChange) {
        scheduleRefresh(pendingChange);
      }
    };

    const runCheck = () => {
      if (document.visibilityState === "hidden" || checkInFlight) {
        return;
      }

      const requestGeneration = generation;
      const check = Promise.resolve(checkerRef.current())
        .then((status) => {
          if (requestGeneration !== generation) {
            if (status.changed) {
              pendingChangeRef.current = status;
              pendingReplayRef.current?.();
            }
            return;
          }
          if (status.changed && document.visibilityState === "hidden") {
            pendingChangeRef.current = status;
          } else if (status.changed) {
            scheduleRefresh(status);
          }
        })
        .catch(() => undefined)
        .finally(() => {
          checkInFlight = null;
        });
      checkInFlight = check;
    };

    const handleFocus = () => runCheck();
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        replayPending();
        runCheck();
      }
    };

    generation += 1;
    pendingReplayRef.current = replayPending;
    replayPending();
    runCheck();
    const intervalId = window.setInterval(runCheck, pollIntervalMs);
    window.addEventListener("focus", handleFocus);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      generation += 1;
      window.clearInterval(intervalId);
      window.removeEventListener("focus", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (pendingReplayRef.current === replayPending) {
        pendingReplayRef.current = null;
      }
      if (pendingStatus?.changed) {
        pendingChangeRef.current = pendingStatus;
      }
      if (debounceTimer !== null) {
        window.clearTimeout(debounceTimer);
      }
      pendingStatus = null;
      checkInFlight = null;
      refreshInFlight = null;
    };
  }, [debounceMs, enabled, pollIntervalMs]);
}

export type ScrollPosition = {
  x: number;
  y: number;
  containers: Array<{
    element: HTMLElement;
    left: number;
    top: number;
  }>;
};

export function captureScrollPosition(): ScrollPosition {
  return {
    x: typeof window === "undefined" ? 0 : window.scrollX,
    y: typeof window === "undefined" ? 0 : window.scrollY,
    containers:
      typeof document === "undefined"
        ? []
        : Array.from(
            document.querySelectorAll<HTMLElement>("[data-external-scroll]"),
            (element) => ({
              element,
              left: element.scrollLeft,
              top: element.scrollTop,
            }),
          ),
  };
}

export function restoreScrollPosition(position: ScrollPosition) {
  if (typeof window === "undefined") {
    return;
  }

  // Do not jump over a newer user scroll/navigation that happened while the
  // async refresh was in flight. Only restore a position that is unchanged
  // since capture.
  if (
    typeof window.scrollTo === "function" &&
    window.scrollX === position.x &&
    window.scrollY === position.y
  ) {
    try {
      window.scrollTo(position.x, position.y);
    } catch {
      // jsdom and embedded webviews may not implement scrolling; preserving
      // the snapshot remains best-effort and must never break a refresh.
    }
  }

  for (const container of position.containers) {
    if (
      container.element.scrollLeft === container.left &&
      container.element.scrollTop === container.top
    ) {
      container.element.scrollLeft = container.left;
      container.element.scrollTop = container.top;
    }
  }
}
