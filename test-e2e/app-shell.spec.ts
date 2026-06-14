import { expect, test } from "@playwright/test";
import { installMockBackend } from "./fixtures/mock-backend";

test("desktop shell uses mock workflow data without exposing API testing mode", async ({ page }) => {
  await installMockBackend(page);
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Screenshots" })).toBeVisible();
  await expect(page.locator(".terminal-card", { hasText: "Area18 TDD · Area18" })).toBeVisible();

  await expect(page.getByText("Submit mode")).toHaveCount(0);
  await expect(page.getByRole("option", { name: "Testing" })).toHaveCount(0);
  await expect(page.getByRole("option", { name: "Production" })).toHaveCount(0);

  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByLabel("AI model")).toHaveValue("qwen3.5-4b");

  const shellMetrics = await page.evaluate(() => {
    const titlebar = document.querySelector(".titlebar");
    const button = document.querySelector(".titlebar button");
    return {
      bodyMinWidth: getComputedStyle(document.body).minWidth,
      bodyMinHeight: getComputedStyle(document.body).minHeight,
      titlebarAppRegion: titlebar ? getComputedStyle(titlebar).getPropertyValue("app-region") : null,
      buttonAppRegion: button ? getComputedStyle(button).getPropertyValue("app-region") : null,
      hasHorizontalOverflow: document.documentElement.scrollWidth > window.innerWidth,
    };
  });

  expect(shellMetrics).toEqual({
    bodyMinWidth: "980px",
    bodyMinHeight: "600px",
    titlebarAppRegion: "drag",
    buttonAppRegion: "no-drag",
    hasHorizontalOverflow: false,
  });
});
