import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

import App from "./App";
import { getPathSettings, setPathSettings } from "./backend";

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
  beforeEach(async () => {
    installLocalStorageMock();
    window.localStorage.clear();
    window.history.replaceState({}, "", "/");
    await setPathSettings({
      wordTargetEnabled: false,
      wordTarget: 500,
      gauntletModeEnabled: false,
    });
  });

  test("shows six all-time journal records at the top of the Dashboard", async () => {
    render(<App />);

    const bestOf = await screen.findByRole("region", { name: "Simply the Best!" });
    expect(within(bestOf).getAllByRole("article")).toHaveLength(6);
    expect(within(bestOf).getByText("Most capsules in a day")).toBeInTheDocument();
    expect(within(bestOf).getByText("Most words in a day")).toBeInTheDocument();
    expect(within(bestOf).getByText("Most tags on one capsule")).toBeInTheDocument();
    expect(within(bestOf).getByText("Longest capsule")).toBeInTheDocument();
    expect(within(bestOf).getByText("Biggest month by capsules")).toBeInTheDocument();
    expect(within(bestOf).getByText("Biggest month by words")).toBeInTheDocument();
    const totalWordsLabel = screen.getByText("Total words");
    await waitFor(() => {
      expect(totalWordsLabel.nextElementSibling).not.toHaveTextContent("Unknown");
    });
    expect(screen.queryByText("This year")).not.toBeInTheDocument();
  });

  test("shows per-entry and per-thread word counts across journal browsing views", async () => {
    render(<App />);

    expect((await screen.findAllByTitle("Word count")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Entries" }));
    const entriesView = await screen.findByRole("region", { name: "Entries" });
    expect((await within(entriesView).findAllByTitle("Word count")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    const searchView = await screen.findByRole("region", { name: "Search" });
    expect((await within(searchView).findAllByTitle("Word count")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Threads" }));
    const threadsView = await screen.findByRole("region", { name: "Threads" });
    expect(await within(threadsView).findByTitle("Thread word count")).toHaveTextContent(
      "44 words",
    );
    expect((await within(threadsView).findAllByTitle("Word count")).length).toBeGreaterThan(0);
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

  test("shows live word targets and blocks saving in Gauntlet mode", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    await screen.findByText(/path_settings\.json/);

    const enableTargets = await screen.findByLabelText("Enable word targets");
    fireEvent.click(enableTargets);
    fireEvent.change(screen.getByLabelText("Word target"), { target: { value: "5" } });
    fireEvent.click(screen.getByLabelText("Gauntlet mode"));
    const saveWritingSettings = screen.getByRole("button", { name: "Save writing settings" });
    fireEvent.click(saveWritingSettings);

    await screen.findByText(/Saved local settings:/);
    await waitFor(() => expect(saveWritingSettings).toBeEnabled());
    expect(await getPathSettings()).toMatchObject({
      wordTargetEnabled: true,
      wordTarget: 5,
      gauntletModeEnabled: true,
    });
    fireEvent.click(screen.getByRole("button", { name: "New Entry" }));

    const composerText = await screen.findByPlaceholderText("Write the entry");
    expect(screen.getByText("0 / 5 words · Gauntlet")).toBeInTheDocument();

    fireEvent.change(composerText, { target: { value: "one two three four" } });
    expect(screen.getByText("4 / 5 words · Gauntlet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();

    fireEvent.keyDown(window, { ctrlKey: true, key: "s" });
    expect(await screen.findByText(/Gauntlet mode requires 5 words/)).toBeInTheDocument();

    const writerButtons = screen.getAllByRole("button", { name: "Writer" });
    fireEvent.click(writerButtons[writerButtons.length - 1]);

    expect(await screen.findByText("4 / 5 words · Gauntlet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText("Write"), {
      target: { value: "one two three four five" },
    });
    expect(screen.getByText("5 / 5 words · Gauntlet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    await waitFor(() =>
      expect(screen.queryByText(/Gauntlet mode requires 5 words/)).not.toBeInTheDocument(),
    );
  });

  test("adds an entry to the end of a selected thread", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Threads" }));
    fireEvent.click(await screen.findByRole("button", { name: "Add entry" }));

    await screen.findByRole("heading", { level: 2, name: "New Entry" });
    expect(screen.getByLabelText("Continue from UUID")).toHaveValue("entry_2490ytiy");
    expect(screen.getByLabelText("Mood")).toHaveValue("calm");
  });

  test("adds an existing standalone entry to a selected thread", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Threads" }));
    fireEvent.click(await screen.findByRole("button", { name: "Add existing" }));

    const dialog = await screen.findByRole("dialog", { name: "Add existing entry" });
    fireEvent.click(
      await within(dialog).findByRole("button", { name: "Add Phase 1 shape to thread" }),
    );

    await waitFor(
      () =>
        expect(
          screen.queryByRole("dialog", { name: "Add existing entry" }),
        ).not.toBeInTheDocument(),
      { timeout: 3000 },
    );
    expect(screen.getByRole("heading", { name: "Phase 1 shape" })).toBeInTheDocument();
  });

  test("clears the thread continuation when starting a standalone entry", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Threads" }));
    fireEvent.click(await screen.findByRole("button", { name: "Add entry" }));

    const entryText = screen.getByPlaceholderText("Write the entry");
    fireEvent.change(entryText, { target: { value: "Keep this standalone draft." } });
    expect(screen.getByLabelText("Continue from UUID")).not.toHaveValue("");

    fireEvent.click(screen.getByRole("button", { name: "Entries" }));
    await screen.findByRole("heading", { level: 2, name: "Entries" });
    fireEvent.click(screen.getByRole("button", { name: "New" }));

    await screen.findByRole("heading", { level: 2, name: "New Entry" });
    expect(screen.getByPlaceholderText("Write the entry")).toHaveValue(
      "Keep this standalone draft.",
    );
    expect(screen.getByLabelText("Continue from UUID")).toHaveValue("");
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

  test("opens Wrapped and browses completed periods", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Wrapped" }));

    const latestHeading = await screen.findByRole("heading", {
      name: /^Capsule Wrapped:/,
    });
    const latestTitle = latestHeading.textContent;
    const newerButton = screen.getByRole("button", { name: "Newer" });
    expect(newerButton).toBeDisabled();
    expect(screen.getByRole("tab", { name: "Month" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    fireEvent.click(screen.getByRole("button", { name: "Older" }));

    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: /^Capsule Wrapped:/ }).textContent,
      ).not.toBe(latestTitle);
    });
    expect(screen.getByRole("button", { name: "Newer" })).toBeEnabled();

    fireEvent.click(screen.getByRole("tab", { name: "Year" }));

    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "Year" })).toHaveAttribute(
        "aria-selected",
        "true",
      ),
    );
  });
});
