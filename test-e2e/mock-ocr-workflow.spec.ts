import { expect, test } from "@playwright/test";
import { installMockBackend } from "./fixtures/mock-backend";

test("mock OCR workflow extracts editable commodity rows", async ({ page }) => {
  await installMockBackend(page);
  await page.goto("/");

  await page.locator('input[type="checkbox"]').first().check();
  await page.getByRole("button", { name: "OCR Selected" }).click();

  await expect(page.locator('input[value="Agricium"]')).toBeVisible();
  await expect(page.locator('input[value="Processed Food"]')).toBeVisible();
  await expect(page.getByText("Processed 1 screenshot(s)")).toBeVisible();

  await expect(page.getByText("Side").first()).toBeVisible();
  await expect(page.getByText("Price / SCU").first()).toBeVisible();
  await expect(page.getByText("SCU", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Status").first()).toBeVisible();
  await expect(page.getByText("Buy SCU")).toHaveCount(0);
  await expect(page.getByText("Sell SCU")).toHaveCount(0);
});
