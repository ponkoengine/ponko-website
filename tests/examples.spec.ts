import { test, expect, type Page } from "@playwright/test";
import { PNG } from "pngjs";
async function gameFrame(page: Page) {
  await expect(
    page.locator("article:not([hidden]) [data-game-status]"),
  ).toContainText("Running", { timeout: 50_000 });
  const element = await page.locator("iframe").elementHandle();
  const frame = (await element!.contentFrame())!;
  await frame.locator("canvas").click();
  return frame;
}
test("view controls show the actual source and unload a running game", async ({
  page,
}) => {
  await page.goto("/");
  const example = page.locator('[data-example="asteroids"]');
  await expect(example.locator('[data-code-file="0"]')).toContainText(
    "pub struct Asteroids",
  );
  await expect(page.locator("iframe")).toHaveCount(0);
  await example.getByRole("button", { name: "Play Asteroids" }).click();
  await gameFrame(page);
  await example.getByRole("button", { name: "Code", exact: true }).click();
  await expect(page.locator("iframe")).toHaveCount(0);
  await expect(example.locator(".play-pane")).toBeHidden();
  await example.getByRole("button", { name: "Split", exact: false }).click();
  await expect(example.locator(".play-pane")).toBeVisible();
  await example.locator("[data-file]").selectOption("1");
  await expect(example.locator('[data-code-file="1"]')).toContainText(
    "Game::new",
  );
});
for (const game of ["asteroids", "breakout", "arena"])
  test(`${game} is compiled WASM, takes input, scores and restarts`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto("/");
    await page.getByRole("tab", { name: new RegExp(game, "i") }).click();
    const example = page.locator(`[data-example="${game}"]`);
    await example.getByRole("button", { name: "Play", exact: true }).click();
    const wasm = page.waitForResponse(
      (r) => r.url().endsWith("game_bg.wasm") && r.ok(),
    );
    await example.locator("[data-launch]").click();
    const response = await wasm;
    const bytes = await response.body();
    expect([...bytes.subarray(0, 4)]).toEqual([0, 97, 115, 109]);
    const frame = await gameFrame(page);
    await page.keyboard.down("Space");
    if (game === "asteroids") await page.keyboard.down("ArrowRight");
    await expect
      .poll(() => frame.evaluate(() => (window as any).__ponkoState?.score), {
        timeout: 25_000,
      })
      .toBeGreaterThan(0);
    if (game === "asteroids") await page.keyboard.up("ArrowRight");
    await page.keyboard.up("Space");
    const screenshot = await frame
      .locator("canvas")
      .screenshot(
        process.env.UPDATE_SCREENSHOTS
          ? { path: `public/screens/${game}.png` }
          : {},
      );
    const png = PNG.sync.read(screenshot),
      colors = new Set();
    for (let y = 60; y < png.height - 60; y += 4)
      for (let x = 10; x < png.width - 10; x += 4) {
        const i = (y * png.width + x) * 4;
        colors.add(`${png.data[i]},${png.data[i + 1]},${png.data[i + 2]}`);
      }
    expect(
      colors.size,
      "The canvas must display rendered gameplay",
    ).toBeGreaterThan(3);
    await frame.getByRole("button", { name: "Restart", exact: true }).click();
    await expect
      .poll(() => frame.evaluate(() => (window as any).__ponkoState?.score))
      .toBe(0);
    await expect
      .poll(() => frame.evaluate(() => (window as any).__ponkoState?.lives))
      .toBe(game === "arena" ? 5 : 3);
    await frame.getByRole("button", { name: "Pause", exact: true }).click();
    await expect(frame.locator("#pause-note")).toBeVisible();
    await frame.getByRole("button", { name: "Resume", exact: true }).click();
    await expect(frame.locator("#pause-note")).toBeHidden();
    await page
      .getByRole("tab", {
        name: game === "asteroids" ? /Breakout/ : /Asteroids/,
      })
      .click();
    await expect(page.locator("iframe")).toHaveCount(0);
    expect(errors).toEqual([]);
  });
test("mobile split view stacks and tabs work from the keyboard", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const example = page.locator('[data-example="asteroids"]');
  const source = await example.locator(".source-pane").boundingBox();
  const play = await example.locator(".play-pane").boundingBox();
  expect(source!.y).toBeGreaterThan(play!.y);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(390);
  await page.getByRole("tab", { name: /Asteroids/ }).focus();
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: /Arena/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await page.screenshot({ path: ".cache/mobile.png", fullPage: true });
});
test("failed WASM downloads show a retry that can launch successfully", async ({
  page,
}) => {
  await page.goto("/");
  await page.route("**/game_bg.wasm", (route) => route.abort());
  await page.getByRole("button", { name: "Play Asteroids" }).click();
  await expect(page.getByRole("button", { name: "Retry game" })).toBeVisible({
    timeout: 50_000,
  });
  await page.unroute("**/game_bg.wasm");
  await page.getByRole("button", { name: "Retry game" }).click();
  await gameFrame(page);
});
test("a slow game launched then scrolled away stays offscreen and does not steal focus", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 300 });
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  await page.route("**/play/asteroids/", async (route) => {
    await gate;
    await route.continue();
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Play Asteroids" }).click();
  await page.evaluate(() =>
    window.scrollTo(0, document.documentElement.scrollHeight),
  );
  await expect
    .poll(() =>
      page
        .locator('[data-example="asteroids"] .play-pane')
        .evaluate((e) => e.getBoundingClientRect().bottom),
    )
    .toBeLessThan(0);
  release();
  await expect(page.locator("iframe")).toHaveCount(1);
  const frame = (await (await page
    .locator("iframe")
    .elementHandle())!.contentFrame())!;
  await frame.waitForFunction(() => (window as any).__ponkoReady);
  await expect
    .poll(() => frame.evaluate(() => (window as any).__ponkoOffscreen))
    .toBe(true);
  expect(await page.evaluate(() => document.activeElement?.tagName)).not.toBe(
    "IFRAME",
  );
});
