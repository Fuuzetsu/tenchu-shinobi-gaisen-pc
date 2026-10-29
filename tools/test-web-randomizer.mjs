import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-randomizer";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", "--enable-gpu", "--use-angle=vulkan",
    "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
await context.grantPermissions(["clipboard-read", "clipboard-write"]);
const page = await context.newPage();
const errors = [];
page.on("pageerror", error => errors.push(error.message));
page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
const stored = name => page.evaluate(name => {
  const bytes = localStorage.getItem(`tenchu-pc/${name}`);
  return bytes == null ? null : JSON.parse(new TextDecoder().decode(new Uint8Array(JSON.parse(bytes))));
}, name);
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const click = async (x, y) => { await page.mouse.click(x, y); await page.waitForTimeout(400); };
const key = async key => { await page.keyboard.press(key); await page.waitForTimeout(400); };
const clipboard = () => page.evaluate(() => navigator.clipboard.readText());
const paste = async text => {
  await click(120, 310);
  await key("Control+a");
  await page.evaluate(text => navigator.clipboard.writeText(text), text);
  await key("Control+v");
  await click(200, 267);
};
const boot = async () => {
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true"
    && document.body.dataset.assetsReady === "true", null, { timeout: 120000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
};
try {
  await page.goto(`${base}/?renderer=webgl2`);
  await boot();
  const normalSave = await page.evaluate(() => localStorage.getItem("tenchu-pc/progress.sav"));
  await key("F7");
  await click(185, 318);
  const wild = (await stored("randomizer.json")).setup;
  assert.equal(wild.document.rules.player, "models");
  assert(wild.document.rules.enemy_pool.groups.includes("animals"));
  await click(378, 220);
  await click(80, 267);
  assert.deepEqual(JSON.parse(await clipboard()), wild);

  const shared = structuredClone(wild);
  shared.document.seed = "browser-日本";
  Object.assign(shared.document.rules, {
    stages: "chosen", mission: 1, layout: 1, language: "japanese",
    player: "chosen", character: 1, health: { min: 999, max: 999 },
    enemies: "uniform", enemy_pool: { groups: ["bosses"], blocked: [] },
  });
  await paste(JSON.stringify(shared));
  assert.deepEqual((await stored("randomizer.json")).setup, shared);
  await paste("{}");
  assert.deepEqual((await stored("randomizer.json")).setup, shared);
  await click(80, 267);
  assert.deepEqual(JSON.parse(await clipboard()), shared);
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 60000 });
  await click(85, 726);
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "playing");
  const first = await state();
  assert.equal(first.health, 999);
  assert.equal(first.character, 1);
  assert.equal(first.mission, 1);
  assert.equal(first.fault, "");
  await key("F7");
  await click(157, 188);
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "playing");
  await key("F7");
  await click(243, 188);
  const next = JSON.parse(await clipboard());
  assert.equal(next.round, 1);
  assert.deepEqual(next.document, shared.document);
  const inventory = (await state()).inventory;
  await page.screenshot({ path: `${output}/shared-run.png` });
  await click(351, 188);
  assert.equal((await state()).screen, "frontend");
  assert.equal(await page.evaluate(() => localStorage.getItem("tenchu-pc/progress.sav")), normalSave);

  await page.reload();
  await boot();
  assert.equal((await state()).screen, "launcher");
  assert.deepEqual((await stored("randomizer.json")).current, next);
  await key("F7");
  await click(301, 214);
  // A resumed run can use a different language from the normal title screen.
  if ((await state()).screen !== "playing") {
    await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 60000 });
    await click(301, 214);
  }
  assert.equal((await state()).screen, "playing");
  assert.deepEqual((await state()).inventory, inventory);
  assert.equal((await state()).health, 999);
  assert.equal((await state()).fault, "");
  await page.screenshot({ path: `${output}/resumed-run.png` });
  assert.deepEqual(errors, []);
  console.log("Randomizer: browser clipboard, Unicode seeds, validation, language loading, next roll, resume and campaign save isolation passed.");
} catch (error) {
  await page.screenshot({ path: `${output}/failure.png` });
  throw error;
} finally {
  await browser.close();
}
