import { test, expect } from "@playwright/test";

test.describe("responsive shell", () => {
  test("opens and closes mobile navigation without horizontal overflow", async ({
    page,
  }) => {
    await page.goto("/");

    await expect(
      page.getByRole("button", { name: "Open navigation" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Close navigation" }),
    ).toHaveCount(0);

    const dimensions = await page.evaluate(() => ({
      viewport: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      bodyWidth: document.body.scrollWidth,
    }));

    expect(dimensions.documentWidth).toBeLessThanOrEqual(dimensions.viewport);
    expect(dimensions.bodyWidth).toBeLessThanOrEqual(dimensions.viewport);

    await page.getByRole("button", { name: "Open navigation" }).click();
    await expect(page.locator(".mobile-menu-button")).toHaveAttribute(
      "aria-expanded",
      "true",
    );

    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("button", { name: "Open navigation" }),
    ).toBeVisible();
  });

  test("keeps the join form usable at narrow widths", async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 640 });
    await page.goto("/");

    await page.getByRole("button", { name: "Open navigation" }).click();
    await expect(page.getByLabel("Your name")).toBeVisible();
    await page.getByLabel("Your name").fill("Mobile user");
    await expect(page.getByRole("button", { name: "Join" })).toBeEnabled();
  });
});
