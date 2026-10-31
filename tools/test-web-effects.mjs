import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";
import { installFrameProbe, measureFrames } from "./browser-frames.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/browser-effects-${backend}`;
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
    ...(process.platform === "linux" ? [`--ozone-platform=${process.env.TENCHU_BROWSER_PLATFORM ?? "x11"}`, "--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 760 } });
await context.addInitScript(() => {
  const settings = { pause_on_defocus: false, sharp_text: true, graphics: {
    blood: { enabled: true }, sky: { enabled: true, time: 720, speed: 10 },
    weather: { mode: "Rain", amount: 100 },
    water: { enabled: true },
    models: { characters: 1, foliage: 1 },
    motion: { hair: true, cloth: true, equipment: true, wind: 60 },
  } };
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
await context.addInitScript(installFrameProbe);
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
  await key("Enter"); await key("Space");
  await page.waitForFunction(() => Number(document.body.dataset.tick) >= 150, null, { timeout: 90000 });
  assert.equal(await page.locator("body").getAttribute("data-renderer"), backend === "webgl2" ? "WebGL2" : "WebGPU");
  await shot("rain-daylight");
  const rainFrames = await page.evaluate(measureFrames, 1500);
  await key("F1");
  const frozen = await state();
  for (let i = 0; i < 13; i++) await key("Tab");
  await shot("weather-menu");
  await key("ArrowRight");
  assert.equal((await settings()).graphics.weather.mode, "Snow");
  assert.deepEqual(await state(), frozen, "Weather settings must preserve paused gameplay state");
  await key("F1");
  await page.waitForFunction(tick => Number(document.body.dataset.tick) > tick + 180, frozen.tick, { timeout: 90000 });
  await shot("snow-daylight");
  await key("F1");
  const snowy = await state();
  await key("r");
  assert.equal((await settings()).graphics.weather.mode, "Original");
  await key("Tab"); await key("r");
  assert.equal((await settings()).graphics.water.enabled, false);
  await key("Tab"); await key("r");
  await key("ArrowDown"); await key("r");
  assert.equal((await settings()).graphics.models.characters, 0);
  assert.equal((await settings()).graphics.models.foliage, 0);
  await key("Tab"); await key("r");
  await key("ArrowDown"); await key("r");
  await key("ArrowDown"); await key("r");
  for (let i=0;i<3;i++) await key("ArrowDown");
  await key("r");
  const motion = (await settings()).graphics.motion;
  assert.equal(motion.hair, false); assert.equal(motion.cloth, false);
  assert.equal(motion.equipment, false); assert.equal(motion.wind, 0);
  await key("Shift+Tab"); await key("Shift+Tab");
  await key("Shift+Tab");
  await key("Shift+Tab"); await key("r");
  assert.equal((await settings()).graphics.sky.enabled, false);
  await key("Shift+Tab"); await key("r");
  assert.equal((await settings()).graphics.blood.hit, "original");
  for (let i=0;i<7;i++) { await key("ArrowDown"); await key("r"); }
  const blood = (await settings()).graphics.blood;
  assert.equal(blood.ground, "original"); assert.equal(blood.drips, "off");
  assert.equal(blood.player_death, false); assert.equal(blood.model_stains, false);
  assert.deepEqual(await state(), snowy, "Disabling effects must preserve paused gameplay state");
  await key("F1");
  await shot("effects-disabled");
  assert.equal((await state()).fault, "");
  assert.deepEqual(await page.evaluate(() => window.effectsGpuErrors), []);
  assert.deepEqual(errors, []);
  await writeFile(`${output}/result.json`, JSON.stringify({ backend, rainFrames, state: await state(), errors }, null, 2));
  console.log(`${backend}: weather, daylight, water, model refinement, secondary motion, F1 resets and unchanged paused state passed`);
} catch (error) {
  await writeFile(`${output}/failure.json`, JSON.stringify({ error: String(error), errors,
    gpuErrors: await page.evaluate(() => window.effectsGpuErrors).catch(() => []),
    state: await state().catch(() => null),
  }, null, 2));
  throw error;
} finally {
  await context.close();
  await browser.close();
}
