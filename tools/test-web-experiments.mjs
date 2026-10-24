import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/browser-experiments-${backend}`;
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
    fog: false, blood: { enabled: true }, sky: { enabled: true, time: 720, speed: 0 },
    lighting: { enabled: true, count: 4, shadow_count: 2, resolution: 256 },
    weather: { mode: "Snow", amount: 100, snow: 100, quality: 0 },
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
  await page.waitForFunction(() => Number(document.body.dataset.tick) >= 45, null, { timeout: 90000 });
  await key("F1"); await tab(7); await select(1);
  const frozen = await state();
  const artwork = await shot("artwork-loaded");
  await key("ArrowRight"); assert.equal((await settings()).graphics.asset_pack, false);
  const original = await shot("artwork-off");
  const artworkDifference = await difference(artwork, original);
  assert(artworkDifference.delta > 1, "loaded artwork switch did not change scene textures");
  assert.deepEqual(await state(),frozen);
  await key("ArrowRight");
  await tab(17);
  await key("ArrowRight"); assert.equal((await state()).god_mode,true);
  await select(1); await key("ArrowRight"); assert.equal((await state()).stealth,true);
  await select(3); await key("Enter"); assert.deepEqual((await state()).inventory,[255,...Array(18).fill(10)]);
  await select(4); await key("Enter"); assert.deepEqual((await state()).inventory,Array(19).fill(0));
  await select(10); await key("ArrowRight"); await key("ArrowRight"); assert.equal((await state()).inventory[4],2);
  await select(2); await key("ArrowRight");
  await shot("experiment-menu");
  await key("F1"); await key("e");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).inventory[4] === 1, null, { timeout: 10000 });
  const beforeTeleport = (await state()).position;
  await page.mouse.click(760,640); await page.mouse.move(5,5); await page.waitForTimeout(500);
  const firstTeleport = (await state()).position;
  assert.notDeepEqual(firstTeleport,beforeTeleport,"normal-camera click teleport failed");
  await page.mouse.click(650,620); await page.mouse.move(5,5); await page.waitForTimeout(500);
  assert.notDeepEqual((await state()).position,firstTeleport,"teleport mode must stay enabled after a click");
  await key("Escape"); assert.equal((await state()).screen,"playing");
  await page.waitForFunction(() => Number(document.body.dataset.tick) >= 980, null, { timeout: 180000 });
  await page.keyboard.down("w"); await page.waitForTimeout(1200); await page.keyboard.up("w");
  await shot("snow-tracks");
  await key("F1"); await tab(15);
  const snowy = await state();
  const beforeRefinement = await shot("snow-before-refinement");
  await key("ArrowRight"); assert.equal((await settings()).graphics.models.characters,1);
  const afterRefinement = await shot("snow-after-refinement");
  const snowDifference = await difference(beforeRefinement,afterRefinement);
  assert(Math.abs(snowDifference.a-snowDifference.b) < 5,"model refinement cleared accumulated snow");
  assert.deepEqual(await state(),snowy,"model options changed paused gameplay");
  await tab(17); await key("r"); await select(1); await key("r");
  assert.equal((await state()).god_mode,false); assert.equal((await state()).stealth,false);
  await key("F1");
  assert.equal((await state()).fault,"");
  assert.deepEqual(await page.evaluate(() => window.effectsGpuErrors),[]); assert.deepEqual(errors,[]);
  await writeFile(`${output}/result.json`,JSON.stringify({backend,artworkDifference,snowDifference,state:await state(),errors},null,2));
  console.log(`${backend}: inventory use, god/stealth controls, repeated click teleport, HD artwork comparison and snow preservation passed`);
} catch (error) {
  await page.screenshot({path:`${output}/failure.png`}).catch(()=>{});
  await writeFile(`${output}/failure.json`,JSON.stringify({error:String(error),errors,state:await state().catch(()=>null)},null,2));
  throw error;
} finally {
  await context.close(); await browser.close();
}
