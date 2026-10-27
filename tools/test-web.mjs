import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";
import { installFrameProbe, measureFrames } from "./browser-frames.mjs";
import { installAudioProbe } from "./browser-audio.mjs";
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-check";
const webgl = process.env.TENCHU_WEBGL === "1";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: process.env.TENCHU_HEADLESS === "1",
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  args: ["--no-sandbox", "--mute-audio", ...(!webgl ? ["--enable-unsafe-webgpu", "--ignore-gpu-blocklist"] : []),
    ...(process.platform === "linux" ? ["--ozone-platform=x11", "--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
const errors = [];
const context = await browser.newContext({ viewport: { width: 1280, height: 760 } });
await context.addInitScript(webgl => {
  if (webgl) Object.defineProperty(navigator, "gpu", { value: undefined });
  window.testMedia = { videos: [], gpuErrors: [] };
  const create = document.createElement.bind(document);
  document.createElement = (...args) => {
    const element = create(...args);
    if (args[0] === "video") window.testMedia.videos.push(element);
    return element;
  };
  if (typeof GPUAdapter !== "undefined") {
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      device.addEventListener("uncapturederror", event => window.testMedia.gpuErrors.push(event.error.message));
      device.lost.then(event => { if (event.reason !== "destroyed") window.testMedia.gpuErrors.push(event.message); });
      return device;
    };
  }
}, webgl);
await context.addInitScript(installFrameProbe);
await context.addInitScript(installAudioProbe);
const page = await context.newPage();
page.on("pageerror", error => { errors.push(error.stack); console.error(error.stack); });
page.on("console", message => {
  if (message.type() === "error" || /GL_INVALID_|WebGL: INVALID_/.test(message.text())) {
    errors.push(message.text()); console.error(message.text());
  }
});
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const shot = async (name, scene = false) => {
  const png = await page.screenshot({ path: `${output}/${name}.png` });
  const image = scene ? await page.evaluate(sceneImage, png.toString("base64")) : undefined;
  if (scene) assert(image.colors > 64, `${name}: scene is blank`);
  if (scene && webgl) assert.equal(await page.evaluate(() => document.getElementById("game").getContext("webgl2").getError()), 0);
  return image;
};
const key = async key => { await page.keyboard.press(key); await page.waitForTimeout(650); };
try {
  await page.goto(base);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 60000 });
  assert.equal(await page.evaluate(() => document.body.dataset.renderer), webgl ? "WebGL2" : "WebGPU");
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 120000 });
  await shot("launcher");
  await key("Enter");
  assert.equal((await state()).screen, "equipment");
  await key("Enter");
  await page.waitForFunction(() => Number(document.body.dataset.tick) > 10);
  const before = await state();
  await page.keyboard.down("w");
  await page.waitForTimeout(1000);
  await page.keyboard.up("w");
  await key("Space");
  const after = await state();
  assert.equal(after.screen, "playing");
  assert(after.tick > before.tick);
  assert.notDeepEqual(after.position, before.position);
  assert.equal(after.fault, "");
  assert(await page.evaluate(() => window.audioProbe.nonSilent), "The mixer must schedule audible samples");
  assert.equal(await page.evaluate(() => window.audioProbe.latencyReads), 0, "Unused hardware latency must not be queried");
  await shot("mission-1", true);
  const downloading = page.waitForEvent("download");
  await key("F9");
  await (await downloading).saveAs(`${output}/download.png`);
  await key("F3");
  const frozen = await state();
  const idle = await page.evaluate(measureFrames, 750);
  assert.equal(idle.frames, 0, "A paused, unchanged scene must retain its image without rendering");
  const frozenDownload = page.waitForEvent("download");
  await key("F9");
  await (await frozenDownload).saveAs(`${output}/paused-download.png`);
  const downloaded = await page.evaluate(sceneImage, (await readFile(`${output}/paused-download.png`)).toString("base64"));
  assert(downloaded.colors > 64, "A screenshot of the paused game must contain its retained image");
  assert.equal((await state()).tick, frozen.tick);
  const original = await shot("frozen-original", true);
  await key("F1"); await key("Tab"); await key("ArrowDown");
  await key("r"); // Compare surface treatments from the Original reference preset.
  for (let i = 0; i < 7; i++) await key("ArrowRight");
  await key("F1");
  const enhanced = await shot("surface-parallax", true);
  assert.deepEqual(enhanced.hud, original.hud, "Post-processing must preserve the life medallion");
  assert.equal(await page.evaluate(() => JSON.parse(new TextDecoder().decode(new Uint8Array(JSON.parse(localStorage.getItem("tenchu-pc/settings.json"))))).graphics.surface_detail), 2);
  await key("F1");
  for (let i = 0; i < 8; i++) await key("ArrowDown");
  await key("ArrowLeft"); await key("F1");
  await shot("surface-bump", true);
  await key("F1"); await key("ArrowLeft"); await key("F1");
  await shot("surface-off", true);
  assert.equal((await state()).tick, frozen.tick);
  assert.equal((await state()).rng, frozen.rng);
  await key("F3");
  await key("F1");
  const paused = await state();
  await page.waitForTimeout(500);
  assert.equal((await state()).tick, paused.tick);
  await shot("settings");
  await page.setViewportSize({ width: 900, height: 620 });
  await page.waitForTimeout(500);
  const size = await page.evaluate(() => {
    const canvas = document.getElementById("game"), main = canvas.parentElement;
    return { width: canvas.getBoundingClientRect().width, target: main.clientWidth,
      height: canvas.getBoundingClientRect().height, targetHeight: main.clientHeight };
  });
  assert(Math.abs(size.width - size.target) < 2);
  assert(Math.abs(size.height - size.targetHeight) < 2);
  await shot("resized");
  await key("F1");
  await key("Escape");
  await key("Escape");
  await key("F6");
  assert.equal((await state()).screen, "launcher");
  await key("ArrowDown"); await key("ArrowDown"); await key("ArrowRight");
  await key("ArrowDown"); await key("ArrowDown");
  for (let i = 0; i < 4; i++) await key("ArrowRight");
  for (let i = 0; i < 3; i++) await key("ArrowUp");
  await key("Enter"); await key("Enter");
  const mission = await state();
  assert.equal(mission.mission, 5);
  assert.equal(mission.character, 1);
  assert.equal(mission.screen, "playing");
  assert.equal(mission.fault, "");
  await shot("mission-5-ayame", true);
  assert(await page.evaluate(() => localStorage.getItem("tenchu-pc/progress.sav") != null));
  await page.reload();
  await page.click("#play");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 60000 });
  await page.waitForFunction(() => window.testMedia.videos.some(video => video.currentTime > 1), null, { timeout: 30000 });
  await shot("opening");
  await key("F1");
  const movieTime = await page.evaluate(() => window.testMedia.videos.at(-1).currentTime);
  await page.waitForTimeout(500);
  assert(Math.abs(await page.evaluate(() => window.testMedia.videos.at(-1).currentTime) - movieTime) < 0.15);
  await key("F1");
  await key("Enter");
  assert(await page.evaluate(() => window.testMedia.videos.every(video => video.paused)));
  await key("Enter");
  await shot("title");
  await key("F6");
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 120000 });
  await key("Enter"); await key("Enter");
  const restored = await state();
  assert.equal(restored.mission, 5);
  assert.equal(restored.character, 1);
  assert.equal(restored.screen, "playing");
  await context.setOffline(true);
  await page.keyboard.down("w");
  await page.waitForTimeout(1500);
  await page.keyboard.up("w");
  const offline = await state();
  assert(offline.tick > restored.tick);
  assert.equal(offline.fault, "");
  await context.setOffline(false);
  assert(await page.evaluate(() => window.audioProbe.contexts.some(context => context.state === "running")));
  await page.click("#fullscreen");
  await page.waitForFunction(() => document.fullscreenElement != null);
  await key("F1");
  assert.equal((await state()).paused, true);
  await key("ArrowRight"); await key("F1");
  assert(await page.evaluate(() => document.fullscreenElement != null));
  await page.evaluate(() => document.exitFullscreen());
  await page.waitForFunction(() => document.fullscreenElement == null);
  assert.deepEqual(await page.evaluate(() => window.testMedia.gpuErrors), []);
  for (let i = 0; i < 6 && await page.evaluate(() => document.body.dataset.ready === "true"); i++) await key("Escape");
  await page.waitForFunction(() => document.body.dataset.ready === "false");
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 60000 });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 120000 });
  assert.equal((await state()).screen, "launcher");
  assert.deepEqual(errors, []);
  const failed = await context.newPage();
  await failed.route("**/data/manifest.json", route => route.fulfill({ status: 503, body: "Unavailable" }));
  await failed.goto(base); await failed.click("#quick");
  await failed.waitForFunction(() => Boolean(document.body.dataset.error));
  assert(await failed.locator("#retry").isVisible());
  await failed.unroute("**/data/manifest.json");
  await failed.click("#retry");
  await failed.waitForFunction(() => !document.body.dataset.error);
  await failed.close();
  console.log(JSON.stringify({ before, after, mission, restored, offline, size }, null, 2));
} finally {
  await shot("last").catch(() => {});
  await browser.close();
}
