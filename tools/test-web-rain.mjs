import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { installFrameProbe, measureFrames } from "./browser-frames.mjs";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/browser-rain-${backend}`;
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
    fog: false, blood: { enabled: true }, sky: { enabled: true, time: 600, speed: 0 },
    lighting: { enabled: true, count: 4, shadow_count: 2, resolution: 256 },
    weather: { mode: "Rain", amount: 100, wetness: 100, puddles: 80, pooling: 1000, reflections: 0, haze: 0, streaks: 100, quality: 0 },
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
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(150); };
let category = 0, row = 0;
const tab = async target => {
  while (category !== target) { await key("Tab"); category = (category + 1) % 18; }
  row = 0;
};
const select = async target => {
  while (row !== target) { await key(row < target ? "ArrowDown" : "ArrowUp"); row += row < target ? 1 : -1; }
};
const shot = async name => {
  const png = await page.screenshot({ path: `${output}/${name}.png` });
  assert((await page.evaluate(sceneImage, png.toString("base64"))).colors > 64, `${name}: blank scene`);
  if (backend === "webgl2") assert.equal(await page.evaluate(() => document.getElementById("game").getContext("webgl2").getError()), 0);
  return png.toString("base64");
};
const difference = async (a,b) => page.evaluate(async ([a,b]) => {
  const pixels = async src => {
    const image = new Image(); image.src = `data:image/png;base64,${src}`; await image.decode();
    const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
    const ctx = canvas.getContext("2d"); ctx.drawImage(image,0,0);
    return ctx.getImageData(850,250,400,450).data;
  };
  const aa = await pixels(a), bb = await pixels(b);
  let delta = 0, brightnessA = 0, brightnessB = 0;
  for (let i=0;i<aa.length;i+=4) { for (let c=0;c<3;c++) { delta += Math.abs(aa[i+c]-bb[i+c]); brightnessA += aa[i+c]; brightnessB += bb[i+c]; } }
  const n = aa.length * 3/4;
  return { delta: delta/n, a: brightnessA/n, b: brightnessB/n };
},[a,b]);
try {
  await page.goto(`${base}/?renderer=${backend}`);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 90000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 180000 });
  await page.locator("#game").click(); await page.mouse.move(5,5);
  await key("Enter"); await key("Enter");
  await page.waitForFunction(() => Number(document.body.dataset.tick) >= 300, null, { timeout: 90000 });
  await key("F1"); await tab(13); await select(7);
  const frozen = await state();
  const matte = await shot("matte");
  for (let i=0;i<20;i++) await key("ArrowRight");
  assert.equal((await settings()).graphics.weather.reflections,200);
  const wet = await shot("reflections");
  const reflectionDifference = await difference(matte,wet);
  assert(reflectionDifference.delta > 1,"wet reflections did not visibly change the ground");
  await select(10);
  for (let i=0;i<16;i++) await key("ArrowRight");
  assert.equal((await settings()).graphics.weather.haze,80);
  const hazy = await shot("rain-haze");
  const hazeDifference = await difference(wet,hazy);
  assert(hazeDifference.delta > 1,"rain haze did not obscure the scene with black fog disabled");
  await select(9); await key("ArrowLeft");
  assert.equal((await settings()).graphics.weather.pooling,975);
  await key("r"); assert.equal((await settings()).graphics.weather.pooling,100);
  await select(11); await key("ArrowRight");
  assert.equal((await settings()).graphics.weather.streaks,110);
  await key("r"); assert.equal((await settings()).graphics.weather.streaks,100);
  await select(17); await key("ArrowRight");
  assert.equal((await settings()).graphics.weather.runoff_amount,75);
  await key("r"); assert.equal((await settings()).graphics.weather.runoff_amount,70);
  assert.deepEqual(await state(),frozen,"rain controls changed paused game state");
  await key("F1"); await shot("storm-playing");
  const rainFrames = await page.evaluate(measureFrames,1500);
  await key("F1");
  await select(0); await key("r");
  assert.equal((await settings()).graphics.weather.mode,"Original");
  await key("F1"); await shot("original-weather");
  const originalFrames = await page.evaluate(measureFrames,1500);
  assert.equal((await state()).fault,"");
  assert.deepEqual(await page.evaluate(() => window.effectsGpuErrors),[]); assert.deepEqual(errors,[]);
  await writeFile(`${output}/result.json`,JSON.stringify({backend,reflectionDifference,hazeDifference,rainFrames,originalFrames,state:await state(),errors},null,2));
  console.log(`${backend}: visible wet reflections and rain haze, pooling/streak/runoff controls, resets and unchanged paused game state passed`);
} catch (error) {
  await page.screenshot({path:`${output}/failure.png`}).catch(()=>{});
  await writeFile(`${output}/failure.json`,JSON.stringify({error:String(error),errors,gpuErrors:await page.evaluate(()=>window.effectsGpuErrors).catch(()=>[]),state:await state().catch(()=>null)},null,2));
  throw error;
} finally {
  await context.close(); await browser.close();
}
