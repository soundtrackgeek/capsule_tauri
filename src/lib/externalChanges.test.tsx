import { act, render } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";

import {
  EXTERNAL_CHANGE_DEBOUNCE_MS,
  EXTERNAL_CHANGE_POLL_INTERVAL_MS,
  captureScrollPosition,
  restoreScrollPosition,
  trackScrollIntent,
  useExternalChanges,
} from "./externalChanges";
import type { ExternalChangeStatus } from "../types";

const unchanged: ExternalChangeStatus = {
  changed: false,
  available: true,
  reopened: false,
  databasePath: "fixture.db",
  reason: null,
};

const changed: ExternalChangeStatus = {
  ...unchanged,
  changed: true,
  reason: "data_version",
};

function Harness({
  checker,
  enabled = true,
  onChange,
}: {
  checker: () => Promise<ExternalChangeStatus>;
  enabled?: boolean;
  onChange: (status: ExternalChangeStatus) => void | Promise<void>;
}) {
  useExternalChanges({ checker, enabled, onChange });
  return null;
}

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("useExternalChanges", () => {
  test("coalesces an in-flight probe and debounces the resulting refresh", async () => {
    vi.useFakeTimers();
    const first = Promise.resolve(unchanged);
    let resolveSecond: ((status: ExternalChangeStatus) => void) | undefined;
    const second = new Promise<ExternalChangeStatus>((resolve) => {
      resolveSecond = resolve;
    });
    const checker = vi
      .fn<() => Promise<ExternalChangeStatus>>()
      .mockReturnValueOnce(first)
      .mockReturnValueOnce(second);
    const onChange = vi.fn();
    render(<Harness checker={checker} onChange={onChange} />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(checker).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_POLL_INTERVAL_MS);
    });
    expect(checker).toHaveBeenCalledTimes(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_POLL_INTERVAL_MS);
    });
    expect(checker).toHaveBeenCalledTimes(2);

    resolveSecond?.(changed);
    await act(async () => {
      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS - 1);
    });
    expect(onChange).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(changed);
  });

  test("rechecks on focus and visible return, but not while hidden", async () => {
    vi.useFakeTimers();
    const checker = vi.fn<() => Promise<ExternalChangeStatus>>().mockResolvedValue(unchanged);
    const onChange = vi.fn();
    const visibilityDescriptor = Object.getOwnPropertyDescriptor(document, "visibilityState");
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });
    const view = render(<Harness checker={checker} onChange={onChange} />);

    await act(async () => {
      await Promise.resolve();
    });
    expect(checker).not.toHaveBeenCalled();

    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    await act(async () => {
      document.dispatchEvent(new Event("visibilitychange"));
      window.dispatchEvent(new Event("focus"));
      await Promise.resolve();
    });
    expect(checker).toHaveBeenCalledTimes(1);

    view.unmount();
    Object.defineProperty(document, "visibilityState", visibilityDescriptor ?? {
      configurable: true,
      value: "visible",
    });
  });

  test("ignores a stale response after the relevant view is disabled", async () => {
    vi.useFakeTimers();
    let resolveProbe: ((status: ExternalChangeStatus) => void) | undefined;
    const checker = vi.fn<() => Promise<ExternalChangeStatus>>().mockReturnValue(
      new Promise((resolve) => {
        resolveProbe = resolve;
      }),
    );
    const onChange = vi.fn();
    const view = render(<Harness checker={checker} onChange={onChange} />);

    view.rerender(<Harness checker={checker} enabled={false} onChange={onChange} />);
    resolveProbe?.(changed);
    await act(async () => {
      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS);
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  test("replays a changed probe when the view returns after a disabled in-flight read", async () => {
    vi.useFakeTimers();
    let resolveProbe: ((status: ExternalChangeStatus) => void) | undefined;
    const checker = vi
      .fn<() => Promise<ExternalChangeStatus>>()
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveProbe = resolve;
        }),
      )
      .mockResolvedValue(unchanged);
    const onChange = vi.fn();
    const view = render(<Harness checker={checker} onChange={onChange} />);

    view.rerender(<Harness checker={checker} enabled={false} onChange={onChange} />);
    resolveProbe?.(changed);
    await act(async () => {
      await Promise.resolve();
    });
    view.rerender(<Harness checker={checker} onChange={onChange} />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS);
    });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(changed);
  });

  test("replays a changed probe that resolves after the view is re-enabled", async () => {
    vi.useFakeTimers();
    let resolveProbe: ((status: ExternalChangeStatus) => void) | undefined;
    const checker = vi
      .fn<() => Promise<ExternalChangeStatus>>()
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveProbe = resolve;
        }),
      )
      .mockResolvedValue(unchanged);
    const onChange = vi.fn();
    const view = render(<Harness checker={checker} onChange={onChange} />);

    view.rerender(<Harness checker={checker} enabled={false} onChange={onChange} />);
    view.rerender(<Harness checker={checker} onChange={onChange} />);
    resolveProbe?.(changed);
    await act(async () => {
      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS);
    });

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(changed);
  });

  test("preserves a debounced change when the view is disabled before flush", async () => {
    vi.useFakeTimers();
    const checker = vi
      .fn<() => Promise<ExternalChangeStatus>>()
      .mockResolvedValueOnce(changed)
      .mockResolvedValue(unchanged);
    const onChange = vi.fn();
    const view = render(<Harness checker={checker} onChange={onChange} />);

    await act(async () => {
      await Promise.resolve();
    });
    expect(onChange).not.toHaveBeenCalled();

    view.rerender(<Harness checker={checker} enabled={false} onChange={onChange} />);
    view.rerender(<Harness checker={checker} onChange={onChange} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS);
    });

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(changed);
  });

  test("defers a changed result received while hidden until visible", async () => {
    vi.useFakeTimers();
    let resolveProbe: ((status: ExternalChangeStatus) => void) | undefined;
    const checker = vi.fn<() => Promise<ExternalChangeStatus>>().mockReturnValue(
      new Promise((resolve) => {
        resolveProbe = resolve;
      }),
    );
    const onChange = vi.fn();
    const visibilityDescriptor = Object.getOwnPropertyDescriptor(document, "visibilityState");
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    render(<Harness checker={checker} onChange={onChange} />);
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });
    resolveProbe?.(changed);
    await act(async () => {
      await Promise.resolve();
    });
    expect(onChange).not.toHaveBeenCalled();

    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    await act(async () => {
      document.dispatchEvent(new Event("visibilitychange"));
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_DEBOUNCE_MS);
    });
    expect(onChange).toHaveBeenCalledTimes(1);
    Object.defineProperty(document, "visibilityState", visibilityDescriptor ?? {
      configurable: true,
      value: "visible",
    });
  });

  test("cleans up polling and focus listeners on unmount", async () => {
    vi.useFakeTimers();
    const checker = vi.fn<() => Promise<ExternalChangeStatus>>().mockResolvedValue(unchanged);
    const onChange = vi.fn();
    const view = render(<Harness checker={checker} onChange={onChange} />);
    await act(async () => {
      await Promise.resolve();
    });
    view.unmount();

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      await vi.advanceTimersByTimeAsync(EXTERNAL_CHANGE_POLL_INTERVAL_MS * 2);
    });
    expect(checker).toHaveBeenCalledTimes(1);
  });
});

describe("external refresh scroll snapshots", () => {
  test("captures and restores the window scroll position without requiring a browser", () => {
    const scrollTo = vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    Object.defineProperty(window, "scrollX", { configurable: true, value: 12 });
    Object.defineProperty(window, "scrollY", { configurable: true, value: 480 });

    const snapshot = captureScrollPosition();
    restoreScrollPosition(snapshot);

    expect(snapshot).toMatchObject({ x: 12, y: 480, containers: [] });
    expect(scrollTo).toHaveBeenCalledWith(12, 480);
  });

  test("does not override a newer user scroll in a tracked container", () => {
    const scrollTo = vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    Object.defineProperty(window, "scrollX", { configurable: true, value: 0 });
    Object.defineProperty(window, "scrollY", { configurable: true, value: 0 });
    const container = document.createElement("div");
    container.dataset.externalScroll = "entries";
    container.scrollTop = 100;
    document.body.append(container);

    const snapshot = captureScrollPosition();
    container.scrollTop = 260;
    restoreScrollPosition(snapshot);

    expect(container.scrollTop).toBe(260);
    expect(scrollTo).toHaveBeenCalledTimes(1);
    container.remove();
  });

  test("restores a tracked container after a layout reset", () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const container = document.createElement("div");
    container.dataset.externalScroll = "entries";
    container.scrollTop = 100;
    document.body.append(container);

    const snapshot = captureScrollPosition();
    container.scrollTop = 0;
    restoreScrollPosition(snapshot);

    expect(container.scrollTop).toBe(100);
    container.remove();
  });

  test("restores a remounted tracked container by marker and occurrence", () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const original = document.createElement("div");
    original.dataset.externalScroll = "entries";
    original.scrollTop = 100;
    document.body.append(original);

    const snapshot = captureScrollPosition();
    original.remove();
    const remounted = document.createElement("div");
    remounted.dataset.externalScroll = "entries";
    document.body.append(remounted);
    restoreScrollPosition(snapshot);

    expect(remounted.scrollTop).toBe(100);
    remounted.remove();
  });

  test("keeps a newer user wheel scroll instead of restoring the snapshot", () => {
    const container = document.createElement("div");
    container.dataset.externalScroll = "entries";
    container.scrollTop = 100;
    document.body.append(container);

    const snapshot = captureScrollPosition();
    const tracker = trackScrollIntent();
    container.scrollTop = 260;
    container.dispatchEvent(new Event("wheel", { bubbles: true }));
    restoreScrollPosition(snapshot, tracker);

    expect(container.scrollTop).toBe(260);
    tracker.dispose();
    container.remove();
  });
});
