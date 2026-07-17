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
