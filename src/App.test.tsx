import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, test } from "vitest";

import App from "./App";

const writerSettingsStorageKey = "capsule-tauri-writer-settings-v1";

function installLocalStorageMock() {
  const store = new Map<string, string>();
  const storage = {
    get length() {
      return store.size;
    },
    clear: () => store.clear(),
    getItem: (key: string) => store.get(key) ?? null,
    key: (index: number) => [...store.keys()][index] ?? null,
    removeItem: (key: string) => {
      store.delete(key);
    },
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  } as Storage;

  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
}

describe("App Writer settings", () => {
  beforeEach(() => {
    installLocalStorageMock();
    window.localStorage.clear();
  });

  test("restores and persists Retro CRT display preferences", async () => {
    window.localStorage.setItem(
      writerSettingsStorageKey,
      JSON.stringify({
        background: "#f7f6f0",
        color: "#17201b",
        fontFamily: "Georgia, ui-serif, serif",
        fontSize: 28,
        lineSpacing: 1.75,
        presentation: "retro",
        retroThemeId: "status-bar-green",
      }),
    );

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Writer" }));

    const presentationSelect = (await screen.findByLabelText(
      "Writer presentation",
    )) as HTMLSelectElement;

    await waitFor(() => expect(presentationSelect.value).toBe("retro"));

    const retroThemeSelect = (await screen.findByLabelText("Retro theme")) as HTMLSelectElement;
    expect(retroThemeSelect.value).toBe("status-bar-green");
    expect((screen.getByTitle("Font size") as HTMLInputElement).value).toBe("28");

    fireEvent.change(retroThemeSelect, { target: { value: "amber-ruler" } });

    await waitFor(() => {
      const savedSettings = JSON.parse(
        window.localStorage.getItem(writerSettingsStorageKey) ?? "{}",
      ) as Record<string, unknown>;

      expect(savedSettings).toMatchObject({
        fontSize: 28,
        presentation: "retro",
        retroThemeId: "amber-ruler",
      });
    });
  });
});
