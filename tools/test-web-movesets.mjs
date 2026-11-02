import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/browser-movesets-${backend}`;
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
    ...(process.platform === "linux" ? [`--ozone-platform=${process.env.TENCHU_BROWSER_PLATFORM ?? "x11"}`, "--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 760 } });
await context.addInitScript(() => {
  const settings = {
    pause_on_defocus: false, sharp_text: true,
    model_moveset: true, moveset_model: "disc:135",
    model_moveset_options: { player_movement: true, no_idle_taunts: true },
    graphics: { asset_pack: true, interpolation: true, smooth_models: true, models: { heroes: true } },
  };
  localStorage.setItem("tenchu-pc/settings.json", JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
  window.effectsGpuErrors = [];
  if (typeof GPUAdapter !== "undefined") {
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      device.addEventListener("uncapturederror", e => window.effectsGpuErrors.push(e.error.message));
      return device;
    };
  }
});
const page = await context.newPage(), errors = [];
page.on("pageerror", error => errors.push(String(error)));
page.on("console", message => {
  if (message.type() === "error" || /GL_INVALID_|WebGL: INVALID_/.test(message.text())) errors.push(message.text());
});
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const settings = () => page.evaluate(() => JSON.parse(new TextDecoder().decode(new Uint8Array(JSON.parse(localStorage.getItem("tenchu-pc/settings.json"))))));
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(120); };
const shot = async name => {
  const png = await page.screenshot({ path: `${output}/${name}.png` });
  const image = await page.evaluate(sceneImage, png.toString("base64"));
  assert(image.colors > 64, `${name}: scene is blank`);
  if (backend === "webgl2") assert.equal(await page.evaluate(() => document.getElementById("game").getContext("webgl2").getError()), 0);
};
try {
  await page.goto(`${base}/?renderer=${backend}`);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 90000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 180000 });
  await page.locator("#game").click();
  await page.mouse.move(5, 5);
  await key("Enter");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "equipment");
  await page.waitForTimeout(500); await key("Space");
  await page.waitForFunction(() => Number(document.body.dataset.tick) >= 150, null, { timeout: 90000 });
  assert.equal(await page.locator("body").getAttribute("data-renderer"), backend === "webgl2" ? "WebGL2" : "WebGPU");

  await key("F1");
  for (let i=0;i<10;i++) await key("Tab");
  const before = await state();
  await shot("moveset-panel");
  const canvas = await page.locator("#game").boundingBox();
  const click = async (x,y) => {
    await page.mouse.move(canvas.x+x,canvas.y+y);
    await page.waitForTimeout(120);
    await page.mouse.click(canvas.x+x,canvas.y+y,{delay:120});
    await page.waitForTimeout(300);
  };
  await click(470,249);
  await shot("dropdown");
  await click(440,274);
  await page.keyboard.type("disc:0",{delay:80});
  await page.waitForTimeout(300);
  await shot("search");
  await click(440,296);
  assert.equal((await settings()).moveset_model,"disc:0");
  await click(470,249);
  await click(440,274);
  await page.keyboard.press("Control+a");
  await page.keyboard.type("disc:135",{delay:80});
  await page.waitForTimeout(300);
  await shot("search-onikage");
  await click(440,296);
  assert.equal((await settings()).moveset_model,"disc:135");
  assert.equal((await settings()).player_model ?? "","");
  assert.equal((await settings()).model_moveset,true);
  await click(363,223);
  assert.equal((await settings()).model_moveset,false);
  await click(363,223);
  assert.equal((await settings()).model_moveset,true);
  assert.equal((await state()).tick,before.tick);
  assert.equal((await state()).rng,before.rng);
  assert.deepEqual((await state()).position,before.position);
  await key("F1");
  await page.keyboard.down("w");
  await page.waitForFunction(t => JSON.parse(window.port.snapshot()).tick>t+30,before.tick);
  await page.keyboard.up("w");
  await key("j");
  await page.waitForTimeout(400);
  await key("F3");
  await shot("npc-moves-original-hero");
  assert.equal((await state()).fault,"");
  assert.deepEqual(await page.evaluate(() => window.effectsGpuErrors),[]);
  assert.deepEqual(errors,[]);
  await writeFile(`${output}/result.json`,JSON.stringify({backend,before,after:await state(),settings:await settings(),errors},null,2));
  console.log(`${backend}: independent movesets, searchable selector, original hero, restoration and gameplay passed`);

} catch (error) {
  await page.screenshot({ path: `${output}/failure.png` }).catch(() => {});
  await writeFile(`${output}/failure.json`, JSON.stringify({ error: String(error), errors,
    gpuErrors: await page.evaluate(() => window.effectsGpuErrors).catch(() => []),
    state: await state().catch(() => null),
  }, null, 2));
  throw error;
} finally {
  await context.close();
  await browser.close();
}
