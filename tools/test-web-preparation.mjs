import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { installAudioProbe } from "./browser-audio.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-preparation";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
    ...(process.platform === "linux" ? ["--ozone-platform=x11", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
await context.addInitScript(installAudioProbe);
const page = await context.newPage();
const errors = [];
page.on("pageerror", error => errors.push(error.stack));
page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(900); };
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const shot = name => page.screenshot({ path: `${output}/${name}.png` });
try {
  await page.goto(base);
  await page.click("#play");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 60000 });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 120000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await key("Enter"); // Opening movie.
  await key("Enter"); // Title animation.
  await key("Enter"); // Begin mission.
  await key("Enter"); // Rikimaru.
  await page.waitForTimeout(4000);
  await shot("mission-selection");
  await page.evaluate(() => { window.audioProbe.nonSilent = false; });
  await key("Space"); // Start also confirms mission selection.
  assert.equal((await state()).screen, "equipment");
  await page.waitForTimeout(5000);
  const briefing = await state();
  await shot("narrated-briefing");
  assert(await page.evaluate(() => window.audioProbe.nonSilent), "Narration must reach the audio mixer");
  await key("Enter"); // Skip to the shop.
  assert.equal((await state()).tick, briefing.tick, "Briefings must not advance gameplay");
  assert.equal((await state()).rng, briefing.rng);
  await key("Enter"); await key("Enter"); await key("r");
  await shot("shop-one-shuriken");
  const canvas = await page.locator("#game").boundingBox();
  await page.mouse.move(canvas.x + canvas.width / 2 - 130, canvas.y + canvas.height - 14);
  await page.waitForTimeout(200);
  await page.mouse.down();
  await page.waitForTimeout(100);
  await page.mouse.up();
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "playing");
  assert.equal((await state()).inventory[1], 1, "The shop must commit additions minus returns");
  await shot("equipped-mission");
  await key("Escape");
  assert.equal((await state()).screen, "equipment");
  await shot("retry-shop");
  await key("Space");
  assert.equal((await state()).screen, "playing", "Retry must skip narration");
  assert.equal((await state()).inventory[1], 0, "Retry must start a fresh selection");
  await key("Escape"); await key("Escape");
  assert.equal((await state()).screen, "frontend");
  await key("F6");
  for (let i = 0; i < 4; i++) await key("ArrowDown");
  await key("ArrowLeft"); // Mission 1 -> training.
  for (let i = 0; i < 3; i++) await key("ArrowUp");
  await key("Enter");
  const training = await state();
  assert.equal(training.mission, 0);
  assert.equal(training.screen, "playing", "Training must bypass the shop");
  assert.deepEqual(training.inventory.slice(0, 4), [255, 5, 0, 0]);
  await shot("training");
  await key("Escape");
  assert.equal((await state()).screen, "launcher", "Training must remain possible to leave");
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ briefing, training }, null, 2));
} finally {
  await shot("last").catch(() => {});
  await browser.close();
}
