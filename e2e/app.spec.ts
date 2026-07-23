import { expect, type Page, test } from "@playwright/test";

function trackBrowserErrors(page: Page) {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    errors.push(error.message);
  });
  return errors;
}

test("loads the mock dashboard and navigates entries and search", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Write-Safe Journal" })).toBeVisible();
  await expect(page.getByText("Database readable")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Database" })).toBeVisible();

  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Entries" })
    .click();
  await expect(page.getByRole("region", { name: "Entries" })).toBeVisible();

  const firstEntryThumbnail = page.locator(".entry-list .entry-attachment-thumb-button").first();
  await expect(firstEntryThumbnail).toBeVisible();
  await firstEntryThumbnail.click();
  await expect(page.locator(".lightbox")).toBeVisible();
  await expect(page.locator(".lightbox-image")).toBeVisible();
  await page.getByTitle("Close").click();
  await expect(page.locator(".lightbox")).not.toBeVisible();

  const phaseOneCard = page.getByRole("button", { name: /Phase 1 shape/ }).first();
  await expect(phaseOneCard).toBeVisible();
  await phaseOneCard.click();
  await expect(page.locator(".detail-panel")).toContainText(
    "Read-only journal browsing starts to feel real.",
  );

  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Search" })
    .click();
  await expect(page.getByRole("region", { name: "Search" })).toBeVisible();
  await page.getByLabel("Query").fill("tag:codex");
  await expect(page.getByText("tag: codex")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Art note" })).toBeVisible();

  const firstSearchThumbnail = page.locator(".entry-list .entry-attachment-thumb-button").first();
  await expect(firstSearchThumbnail).toBeVisible();
  await firstSearchThumbnail.click();
  await expect(page.locator(".lightbox")).toBeVisible();
  await expect(page.locator(".lightbox-image")).toBeVisible();

  expect(browserErrors).toEqual([]);
});

test("reads the linked entry from Cover Wall", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Cover Wall" })
    .click();

  await expect(page.getByRole("region", { name: "Cover wall" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Linked entry" })).toBeVisible();
  await expect(page.locator(".cover-detail-panel")).toContainText(
    "Read-only journal browsing starts to feel real.",
  );

  expect(browserErrors).toEqual([]);
});

test("creates a journal entry through the composer", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);
  const title = `Playwright smoke ${Date.now()}`;

  await page.goto("/");
  await page.getByRole("button", { name: /^New$/ }).click();

  const composer = page.getByRole("region", { name: "New entry" });

  await expect(composer).toBeVisible();
  await composer.getByRole("textbox", { name: "Title" }).fill(title);
  await composer
    .getByRole("textbox", { name: "Entry" })
    .fill("This entry was created by Playwright to protect the main writing path.");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByRole("heading", { name: "Entries", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: new RegExp(title) })).toBeVisible();
  await expect(page.locator(".detail-panel")).toContainText(title);

  expect(browserErrors).toEqual([]);
});

test("initializes Cloud AI and confirms metadata generation in the composer", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/");
  await page.getByRole("button", { name: /^New$/ }).click();

  const composer = page.getByRole("region", { name: "New entry" });
  await composer
    .getByRole("textbox", { name: "Entry" })
    .fill("A quiet morning walk made the next project decision feel much clearer.");

  const generateButton = composer.getByRole("button", { name: "Generate" });
  await expect(generateButton).toBeEnabled();
  await expect(generateButton).toHaveAttribute(
    "title",
    "Generate title and summary with Gemini / gemini-3.5-flash",
  );
  await generateButton.click();

  const privacyDialog = page.getByRole("dialog", {
    name: "Send this entry to Gemini?",
  });
  await expect(privacyDialog).toBeVisible();
  await expect(privacyDialog).toContainText("Image files and API keys are never sent.");
  await privacyDialog.getByRole("button", { name: "Continue" }).click();

  await expect(privacyDialog).not.toBeVisible();
  await expect(composer.getByRole("heading", { name: "AI Suggestion" })).toBeVisible();
  await expect(page.getByRole("status")).toContainText(
    "Generated title and summary with Gemini / gemini-3.5-flash.",
  );
  await expect(generateButton).toBeEnabled();

  expect(browserErrors).toEqual([]);
});

test("confirms update installation inside the app from the banner and Settings", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/?mock-app-update=0.29.3");

  await expect(page.getByText("Capsule 0.29.3 is available.")).toBeVisible();
  await page.getByRole("button", { name: "Install update" }).click();

  const bannerDialog = page.getByRole("dialog", { name: "Install Capsule 0.29.3?" });
  await expect(bannerDialog).toBeVisible();
  await expect(bannerDialog).toContainText("download and verify the signed update");
  await bannerDialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(bannerDialog).not.toBeVisible();

  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Settings" })
    .click();
  const applicationPanel = page.locator(".panel").filter({
    has: page.getByRole("heading", { name: "Application" }),
  });
  await applicationPanel.getByRole("button", { name: "Install update" }).click();

  const settingsDialog = page.getByRole("dialog", { name: "Install Capsule 0.29.3?" });
  await expect(settingsDialog).toBeVisible();
  await settingsDialog.getByRole("button", { name: "Install update" }).click();

  await expect(settingsDialog).not.toBeVisible();
  await expect(page.getByRole("status")).toContainText(
    "Update installed. Restart Capsule to finish applying it.",
  );
  expect(browserErrors).toEqual([]);
});

test("requires sync safety confirmation before manual sync", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Sync" })
    .click();

  await expect(page.getByRole("heading", { name: "Sync Status" })).toBeVisible();
  await page.getByRole("button", { name: "Review" }).click();

  const dialog = page.getByRole("dialog", { name: "Review Sync Run" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(/verified database backup/)).toBeVisible();

  const runButton = dialog.getByRole("button", { name: "Run sync" });
  await expect(runButton).toBeDisabled();

  await dialog.getByRole("checkbox", { name: /I understand/ }).check();
  await expect(runButton).toBeEnabled();
  await runButton.click();

  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("status")).toContainText("Sync completed");

  expect(browserErrors).toEqual([]);
});

test("configures Cloud AI settings without exposing API keys", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Settings" })
    .click();

  await expect(page.getByRole("heading", { name: "Cloud AI" })).toBeVisible();
  await page.getByLabel("Provider").selectOption("openrouter");
  await page.getByLabel("OpenRouter model").selectOption("deepseek/deepseek-v4-flash");
  await page.getByLabel("Context limit").fill("12");
  const geminiKeyRow = page.locator(".ai-key-row").filter({ hasText: "Google Gemini" });
  await geminiKeyRow.getByLabel("Google Gemini API key").fill("mock-gemini-key");
  await expect(geminiKeyRow).toContainText("Unsaved key entered");

  await page.getByRole("button", { name: "Save Cloud AI" }).click();

  await expect(page.getByRole("status")).toContainText("saved 1 API key");
  await expect(page.locator(".detail-list").filter({ hasText: "Active model" })).toContainText(
    "deepseek/deepseek-v4-flash",
  );

  await expect(geminiKeyRow).toContainText("Configured");
  await expect(geminiKeyRow).not.toContainText("mock-gemini-key");

  expect(browserErrors).toEqual([]);
});

test("adds a mood, edits its sentiment, and removes it", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);
  const moodName = `playwright-mood-${Date.now()}`;
  const moodLabel = moodName
    .split("-")
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(" ");

  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "Settings" })
    .click();

  const moodsPanel = page.locator(".panel").filter({
    has: page.getByRole("heading", { name: "Moods", exact: true }),
  });
  const addMood = moodsPanel.getByRole("region", { name: "Add mood" });
  await addMood.getByLabel("Mood name").fill(moodName);
  await addMood.getByLabel("Sentiment score").fill("0.35");
  await addMood.getByRole("button", { name: "Add", exact: true }).click();

  await expect(page.getByRole("status")).toContainText("Added mood with backup");
  const moodChip = moodsPanel.getByRole("button", {
    name: `Edit ${moodLabel} sentiment`,
  });
  await expect(moodChip).toContainText("+0.35");

  const editSentiment = moodsPanel.getByRole("region", { name: "Edit sentiment" });
  await editSentiment.getByLabel("Sentiment score").fill("-0.25");
  await editSentiment.getByRole("button", { name: "Save", exact: true }).click();

  await expect(page.getByRole("status")).toContainText("Updated mood sentiment with backup");
  await expect(moodChip).toContainText("-0.25");

  await moodsPanel.getByLabel("Delete mood").selectOption(moodName);
  await moodsPanel.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Cleared mood with backup");
  await expect(moodChip).toHaveCount(0);

  expect(browserErrors).toEqual([]);
});

test("uses the AI chat workspace with mock streaming and retry", async ({ page }) => {
  const browserErrors = trackBrowserErrors(page);

  page.on("dialog", (dialog) => dialog.accept());
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Primary" })
    .getByRole("button", { name: "AI" })
    .click();

  await expect(page.getByRole("heading", { name: "Chats" })).toBeVisible();
  await page.getByPlaceholder("Ask about the selected entries").fill("What stands out about Capsule Tauri?");
  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByText(/Previewed \d+ context entries/)).toBeVisible();

  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".ai-transcript")).toContainText("mock streamer");

  await page.getByTitle("New chat").click();
  await expect(page.locator(".ai-transcript")).toContainText("New AI chat");
  await expect(page.locator(".ai-transcript")).not.toContainText("What stands out about Capsule Tauri?");

  await page.getByPlaceholder("Ask about the selected entries").fill("Summarize the Codex workflow note");
  await page.getByRole("button", { name: "Send" }).click();
  await page.getByRole("button", { name: "Stop" }).click();
  await expect(page.locator(".ai-message--user")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Retry" })).toBeVisible();
  await page.getByRole("button", { name: "Retry" }).click();
  await expect(page.locator(".ai-transcript")).toContainText("mock streamer");

  expect(browserErrors).toEqual([]);
});
