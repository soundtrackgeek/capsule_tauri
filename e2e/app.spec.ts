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
