import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { installAudioProbe } from "./browser-audio.mjs";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-death";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
    ...(process.platform === "linux" ? ["--ozone-platform=x11", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
await context.addInitScript(installAudioProbe);
await context.addInitScript(() => {
  const settings = { sharp_text: true, pause_on_defocus: false, pause_in_menu: false,
    wait_after_death: true, graphics: { blood: { enabled: true } } };
  localStorage.setItem("tenchu-pc/settings.json", JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
});
const page = await context.newPage(), errors = [];
page.on("pageerror", error => errors.push(String(error)));
page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(150); };
const shot = async name => {
  const png = await page.screenshot({ path: `${output}/${name}.png` });
  assert((await page.evaluate(sceneImage, png.toString("base64"))).colors > 64, "Scene must remain visible");
  return png;
};
try {
  await page.goto(`${process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000"}/?renderer=${backend}`);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 90000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 180000 });
  await page.locator("#game").click(); await page.mouse.move(5, 5);
  await key("Enter");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "equipment");
  await page.waitForTimeout(1000);
  await key("Space");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).tick >= 300, null, { timeout: 90000 });
  const playing = await state();
  assert.equal(playing.screen, "playing");
  assert.equal(playing.god_mode, false);
  await key("F1");
  for (let i = 0; i < 17; i++) await key("Tab");
  for (let i = 0; i < 4; i++) await key("ArrowDown");
  await key("Enter"); // Clear equipment.
  for (let i = 0; i < 8; i++) await key("ArrowDown");
  for (let i = 0; i < 10; i++) await key("ArrowRight");
  assert.equal((await state()).inventory[6], 10);
  await key("F1"); await key("c");
  for (let attempt = 0; attempt < 10 && (await state()).health > 0; attempt++) {
    await page.waitForFunction(() => {
      const game = JSON.parse(window.port.snapshot());
      return game.health === 0 || [0x500, 0x501].includes(game.motion);
    }, null, { timeout: 15000 });
    if ((await state()).health === 0) break;
    const before = await state();
    await key("e");
    await page.waitForFunction(count => JSON.parse(window.port.snapshot()).inventory[6] < count, before.inventory[6], { timeout: 10000 });
    await page.waitForTimeout(1200);
    await page.keyboard.down("w"); await page.waitForTimeout(350); await page.keyboard.up("w");
    await page.keyboard.down("s"); await page.waitForTimeout(350); await page.keyboard.up("s");
    await page.waitForTimeout(1500);
    console.log("Mine contact", (await state()).health);
  }
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "death-hold", null, { timeout: 90000 });
  await page.waitForTimeout(500);
  const held = await state();
  assert.equal(held.health, 0);
  const heldImage = await shot("held-death-scene");
  await page.evaluate(() => { window.audioProbe.nonSilent = false; });
  for (let i = 0; i < 4; i++) {
    await key("ArrowDown");
    await page.waitForTimeout(5000);
    assert.deepEqual(await state(), held, "Death hold must not advance or navigate a hidden menu");
  }
  assert.equal(await page.evaluate(() => window.audioProbe.nonSilent), false, "No hidden menu music or navigation sounds");
  assert.deepEqual(await shot("held-after-twenty-seconds"), heldImage, "Held effects must stay intact");
  await key("Enter");
  assert.equal((await state()).screen, "game-over");
  await page.waitForFunction(() => window.audioProbe.nonSilent, null, { timeout: 15000 });
  await shot("confirmed-failure-menu");
  await key("Enter");
  assert.equal((await state()).screen, "equipment");
  assert.deepEqual(errors, []);
  await writeFile(`${output}/result.json`, JSON.stringify({ backend, held, errors }, null, 2));
  console.log(`${backend}: death hold freezes gameplay, stays silent, and opens failure music/menu only on confirmation`);
} catch (error) {
  await page.screenshot({ path: `${output}/failure.png` }).catch(() => {});
  await writeFile(`${output}/failure.json`, JSON.stringify({ error: String(error), errors, state: await state().catch(() => null) }, null, 2));
  throw error;
} finally {
  await browser.close();
}
