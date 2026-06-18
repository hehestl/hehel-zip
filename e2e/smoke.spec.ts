import { test, expect } from "@playwright/test";

/**
 * Smoke: dev server must be running (`npm run dev`) for this test.
 * Full Tauri E2E requires WebDriver; this validates UI shell loads.
 */
test("app shell loads", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await expect(page.locator("body")).toBeVisible();
});
