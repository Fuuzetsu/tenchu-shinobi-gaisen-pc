import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-savestate";
await mkdir(output, { recursive: true });
const profile = await mkdtemp(join(tmpdir(), "tenchu-savestate-browser-"));
let browser, page;
const errors = [];
const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(160); };
const shot = async name => {
  const png = await page.screenshot({ path: `${output}/${name}.png` });
  assert((await page.evaluate(sceneImage, png.toString("base64"))).colors > 64);
};
async function open() {
  browser = await chromium.launchPersistentContext(profile, {
    viewport: { width: 1280, height: 800 },
    executablePath: process.env.BROWSER_EXECUTABLE || undefined,
    headless: process.env.TENCHU_HEADLESS === "1",
    args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
      ...(process.platform === "linux" ? ["--ozone-platform=x11", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
  });
  await browser.addInitScript(() => {
    const key = "tenchu-pc/settings.json";
    if (!localStorage.getItem(key)) {
      const settings = { sharp_text: true, pause_on_defocus: false, rewind_seconds: 5, graphics: { interpolation: true } };
      localStorage.setItem(key, JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
    }
  });
  page = await browser.newPage();
  page.on("pageerror", e => errors.push(String(e)));
  page.on("console", e => { if (e.type() === "error") errors.push(e.text()); });
  await page.goto(`${base}/?renderer=${backend}`);
  await page.click("#quick");
  await page.waitForFunction(() => document.body.dataset.ready === "true", null, { timeout: 90000 });
  await page.evaluate(async () => { window.port = await import("./pkg/tenchu_pc.js"); });
  await page.waitForFunction(() => document.body.dataset.assetsReady === "true", null, { timeout: 180000 });
  await page.locator("#game").click();
  await page.mouse.move(5, 5);
}
async function restored(expected) {
  await page.waitForFunction(tick => {
    const s = JSON.parse(window.port.snapshot());
    return s.paused && s.screen === "playing" && s.tick === tick;
  }, expected.tick, { timeout: 30000 });
  assert.deepEqual(await state(), expected);
}
try {
  await open();
  await key("Enter");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).screen === "equipment");
  await page.waitForTimeout(500); await key("Space");
  await page.waitForFunction(() => JSON.parse(window.port.snapshot()).tick >= 70);
  await key("F3");
  const saved = await state();
  await key("F11");
  await page.waitForFunction(async () => {
    const db = await new Promise(resolve => {
      const r = indexedDB.open("tenchu-savestates", 1); r.onsuccess = () => resolve(r.result);
    });
    try {
      if (!db.objectStoreNames.contains("slots")) return false;
      return await new Promise(resolve => {
        const r = db.transaction("slots").objectStore("slots").get(0);
        r.onsuccess = () => resolve(r.result?.length > 1000);
      });
    } finally { db.close(); }
  });
  await shot("saved");
  await key("F3");
  await page.keyboard.down("w");
  await page.waitForFunction(t => JSON.parse(window.port.snapshot()).tick >= t + 30, saved.tick);
  await page.keyboard.up("w");
  await key("F3");
  assert.notDeepEqual((await state()).position, saved.position);
  await key("F12");
  await restored(saved);
  await shot("loaded");
  await browser.close(); browser = null;
  await open();
  assert.equal((await state()).screen, "launcher");
  await key("F12");
  await restored(saved);
  await shot("loaded-after-browser-restart");
  if (process.env.TENCHU_NATIVE_STATE) {
    const bytes = await readFile(process.env.TENCHU_NATIVE_STATE);
    await page.evaluate(async bytes => {
      const db = await new Promise(resolve => {
        const r = indexedDB.open("tenchu-savestates", 1); r.onsuccess = () => resolve(r.result);
      });
      try {
        await new Promise((resolve, reject) => {
          const t = db.transaction("slots", "readwrite");
          t.objectStore("slots").put(new Uint8Array(bytes), 0);
          t.oncomplete = resolve; t.onabort = () => reject(t.error);
        });
      } finally { db.close(); }
    }, Array.from(bytes));
    await key("F12");
    await page.waitForTimeout(3000);
    const imported = await state();
    assert.equal(imported.fault, "");
    assert.notEqual(imported.tick, saved.tick);
    assert.equal(imported.screen, "playing");
    await shot("native-state-loaded-in-browser");
    await writeFile(`${output}/native-import.json`, JSON.stringify(imported, null, 2));
  }
  const loaded = await state();
  await key("Control+F12");
  assert.deepEqual(await state(), loaded, "Loading a file clears rewind history");
  await key("F3");
  await page.keyboard.down("w");
  await page.waitForFunction(t => JSON.parse(window.port.snapshot()).tick >= t + 165, loaded.tick);
  await page.keyboard.up("w");
  await key("F3");
  const before = await state();
  await key("Control+F12");
  const rewound = await state();
  assert(rewound.paused && rewound.tick < before.tick && rewound.tick >= before.tick - 30);
  await key("Control+F12");
  const earlier = await state();
  assert.equal(earlier.tick, rewound.tick - 30);
  await shot("rewound");
  await key("F3");
  await page.keyboard.down("s");
  await page.waitForFunction(t => JSON.parse(window.port.snapshot()).tick >= t + 35, earlier.tick);
  await page.keyboard.up("s");
  await key("Control+F12");
  const branch = await state();
  assert(branch.paused && branch.tick === rewound.tick);
  assert.notDeepEqual(branch.position, rewound.position, "Resuming replaces the abandoned future");
  await shot("new-branch");
  await key("F12");
  await restored(loaded);
  await key("Control+F12");
  await restored(loaded);
  assert.deepEqual(errors, []);
  await writeFile(`${output}/result.json`, JSON.stringify({ saved, before, rewound, earlier, branch, errors }, null, 2));
  console.log(`${backend}: persistent savestates and bounded rewind restore and branch successfully`);
} catch (error) {
  await page?.screenshot({ path: `${output}/failure.png` }).catch(() => {});
  await writeFile(`${output}/failure.json`, JSON.stringify({ error: String(error), errors, state: await state().catch(() => null) }, null, 2));
  throw error;
} finally {
  await browser?.close();
  await rm(profile, { recursive: true, force: true });
}
