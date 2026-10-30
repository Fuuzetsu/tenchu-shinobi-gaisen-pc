import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-artwork";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const manifest = await (await fetch(`${base}/data/manifest.json`)).json();
assert(manifest.artwork?.game.length, "Export an asset pack before running this check");
await mkdir(output, { recursive: true });
const browser = process.env.TENCHU_CDP
  ? await chromium.connectOverCDP(process.env.TENCHU_CDP)
  : await chromium.launch({
    headless: process.env.TENCHU_HEADLESS === "1",
    executablePath: process.env.BROWSER_EXECUTABLE || undefined,
    args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
      ...(process.platform === "linux" ? [
        `--ozone-platform=${process.env.TENCHU_BROWSER_PLATFORM ?? "x11"}`,
        "--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface",
      ] : [])],
  });
const context = await browser.newContext({ viewport: { width: 1280, height: 760 } });
await context.addInitScript(() => {
  const settings = { sharp_text: true, pause_on_defocus: false, graphics: { asset_pack: true } };
  localStorage.setItem("tenchu-pc/settings.json", JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
  window.artworkGpuErrors = [];
  if (typeof GPUAdapter !== "undefined") {
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      device.addEventListener("uncapturederror", e => window.artworkGpuErrors.push(e.error.message));
      return device;
    };
  }
});
let release, held = false;
const gate = new Promise(resolve => { release = resolve; });
const delayed = manifest.artwork.game[0].split("/").map(encodeURIComponent).join("/");
await context.route(`**/data/pack/${delayed}`, async route => {
  held = true;
  await gate;
  await route.continue();
});
const page = await context.newPage(), errors = [], missions = [];
page.on("pageerror", e => errors.push(String(e)));
page.on("console", m => { if (m.type() === "error" || /GL_INVALID_|WebGL: INVALID_/.test(m.text())) errors.push(m.text()); });
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(200); };
const shot = async name => {
  const bytes = await page.screenshot({ path: `${output}/${name}.png` });
  assert((await page.evaluate(sceneImage, bytes.toString("base64"))).colors > 64, `${name}: blank frame`);
  if (backend === "webgl2") assert.equal(await page.evaluate(() => document.getElementById("game").getContext("webgl2").getError()), 0);
  return bytes.toString("base64");
};
async function difference(a, b) {
  return page.evaluate(async ({ a, b }) => {
    const pixels = async encoded => {
      const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
      const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
      const ctx = canvas.getContext("2d"); ctx.drawImage(image, 0, 0);
      return ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    };
    const [left, right] = await Promise.all([pixels(a), pixels(b)]);
    return left.reduce((count, value, i) => count + (Math.abs(value - right[i]) > 4), 0);
  }, { a, b });
}
try {
  await page.goto(`${base}/?renderer=${backend}`);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 90000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.locator("#game").click();
  assert.notEqual(await page.locator("body").getAttribute("data-assets-ready"), "true");
  await shot("menu-during-download");
  release();
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 180000 });
  assert(held, "A mission artwork download must have been delayed");
  assert.equal(await page.locator("body").getAttribute("data-renderer"), backend === "webgl2" ? "WebGL2" : "WebGPU");
  await context.setOffline(true);
  await key("Enter"); await key("Space");
  await page.waitForFunction(() => Number(document.body.dataset.tick) > 10);
  await key("F3");
  const frozen = await state(), hd = await shot("mission-01-hd");
  await key("F1"); for (let i = 0; i < 7; i++) await key("Tab");
  await key("ArrowDown"); await key("ArrowRight"); await key("F1");
  const original = await shot("mission-01-original");
  assert.deepEqual(await state(), frozen, "Artwork changes must preserve gameplay");
  const changed = await difference(hd, original);
  assert(changed > 5000, "The exported pack must visibly replace the mission artwork");
  await key("F1"); await key("ArrowRight"); await key("ArrowDown"); await key("Enter"); await key("F1");
  const restored = await shot("mission-01-restored");
  assert.deepEqual(await state(), frozen, "Cached pack reload must preserve gameplay");
  assert.equal(await difference(hd, restored), 0, "Reload must restore the same image offline");
  await key("F3");
  await page.keyboard.down("w"); await page.waitForTimeout(1000); await page.keyboard.up("w");
  assert.notDeepEqual((await state()).position, frozen.position);
  missions.push(await state());
  for (let mission = 2; mission <= 10; mission++) {
    await key("Escape"); await key("Escape");
    assert.equal((await state()).screen, "launcher");
    await key("ArrowDown"); await key("ArrowRight");
    await key("ArrowDown"); await key("ArrowDown"); await key("ArrowRight");
    for (let i = 0; i < 3; i++) await key("ArrowUp");
    await key("Enter"); await key("Space");
    await page.waitForFunction(() => Number(document.body.dataset.tick) > 10);
    const current = await state();
    assert.equal(current.mission, mission); assert.equal(current.screen, "playing"); assert.equal(current.fault, "");
    await shot(`mission-${String(mission).padStart(2, "0")}-hd`);
    missions.push(current);
    console.log(`Mission ${mission}: offline HD load passed`);
  }
  assert.deepEqual(await page.evaluate(() => window.artworkGpuErrors), []);
  assert.deepEqual(errors, []);
  await writeFile(`${output}/results.json`, JSON.stringify({ backend, changed, frozen, missions, errors }, null, 2));
} finally {
  release();
  await page.screenshot({ path: `${output}/last.png` }).catch(() => {});
  await context.close();
  await browser.close();
}
