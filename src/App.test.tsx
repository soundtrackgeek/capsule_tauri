import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

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
    window.history.replaceState({}, "", "/");
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

  test("opens a fresh Writer draft after saving an edited entry", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Entries" }));
    fireEvent.click(await screen.findByRole("button", { name: /Phase 1 shape/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));

    await screen.findAllByRole("heading", { name: "Edit Entry" });

    const writerButtons = screen.getAllByRole("button", { name: "Writer" });
    fireEvent.click(writerButtons[writerButtons.length - 1]);

    await screen.findByText("Edit");
    fireEvent.change(screen.getByPlaceholderText("Write"), {
      target: { value: "Writer edit saved from regression test." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(
      () => expect(screen.getByRole("heading", { name: "Entries" })).toBeInTheDocument(),
      { timeout: 3000 },
    );

    fireEvent.click(screen.getByRole("button", { name: "Dashboard" }));
    fireEvent.click(screen.getByRole("button", { name: "Writer" }));

    await screen.findByText("New");
    expect((screen.getByPlaceholderText("Write") as HTMLTextAreaElement).value).toBe("");
  });

  test("confirms update installation inside the app without opening a native prompt", async () => {
    window.history.replaceState({}, "", "/?mock-app-update=0.29.3");
    const nativeConfirm = vi.spyOn(window, "confirm");

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Install update" }));

    const dialog = await screen.findByRole("dialog", { name: "Install Capsule 0.29.3?" });
    expect(nativeConfirm).not.toHaveBeenCalled();
    expect(dialog).toHaveTextContent("download and verify the signed update");

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(dialog).not.toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "Install update" }));
    const confirmedDialog = await screen.findByRole("dialog", {
      name: "Install Capsule 0.29.3?",
    });
    fireEvent.click(within(confirmedDialog).getByRole("button", { name: "Install update" }));

    expect(await screen.findByText("Update installed. Restart Capsule to finish applying it.")).toBeVisible();
    expect(nativeConfirm).not.toHaveBeenCalled();
  });
});
