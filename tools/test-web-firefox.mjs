import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { setTimeout as delay } from "node:timers/promises";
import { sceneImage } from "./browser-image.mjs";
import { installFrameProbe, measureFrames } from "./browser-frames.mjs";
import { installAudioProbe } from "./browser-audio.mjs";

const output = process.env.TENCHU_WEB_CAPTURES ?? "private/browser-firefox";
await mkdir(output, { recursive: true });
const driver = spawn("geckodriver", ["--port", "0"], { stdio: ["ignore", "pipe", "pipe"] });
let log = "", session, endpoint;
driver.stderr.on("data", data => { log += data; });
async function request(method, path, data) {
  const response = await fetch(`${endpoint}${path}`, {
    method, headers: { "Content-Type": "application/json" },
    body: data === undefined ? undefined : JSON.stringify(data),
    signal: AbortSignal.timeout(120000),
  });
  const { value } = await response.json();
  if (!response.ok) throw new Error(JSON.stringify(value));
  return value;
}
const command = (method, path, data) => request(method, `/session/${session}${path}`, data);
const run = script => command("POST", "/execute/sync", { script, args: [] });
const state = () => run("return JSON.parse(window.port.snapshot())");
async function click(selector) {
  const element = await command("POST", "/element", { using: "css selector", value: selector });
  await command("POST", `/element/${element["element-6066-11e4-a52e-4f735466cecf"]}/click`, {});
}
async function wait(script, timeout = 60000) {
  const end = Date.now() + timeout;
  while (!await run(`return (${script})`)) {
    const error = await run("return document.body.dataset.error || window.testErrors?.join('\\n')");
    if (error) throw new Error(error);
    if (Date.now() > end) throw new Error(`Timed out: ${script}`);
    await delay(200);
  }
}
async function key(value, hold = 100) {
  await command("POST", "/actions", { actions: [{ type: "key", id: "keyboard", actions: [
    { type: "keyDown", value }, { type: "pause", duration: hold }, { type: "keyUp", value },
  ] }] });
  await delay(800);
}
async function shot(name, scene = false) {
  const encoded = await command("GET", "/screenshot");
  await writeFile(`${output}/${name}.png`, Buffer.from(encoded, "base64"));
  if (scene) {
    const image = await command("POST", "/execute/async", {
      script: `(${sceneImage.toString()})(arguments[0]).then(arguments[1]);`, args: [encoded],
    });
    assert(image.colors > 64, `${name}: scene is blank`);
    assert.equal(await run("return document.getElementById('game').getContext('webgl2').getError()"), 0);
    return image;
  }
}
try {
  endpoint = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(log || "geckodriver did not start")), 15000);
    driver.once("error", reject);
    driver.stdout.on("data", data => {
      log += data;
      const match = log.match(/Listening on (127\.0\.0\.1:\d+)/);
      if (match) { clearTimeout(timer); resolve(`http://${match[1]}`); }
    });
  });
  const created = await request("POST", "/session", { capabilities: { alwaysMatch: {
    browserName: "firefox",
    "moz:firefoxOptions": {
      ...(process.env.FIREFOX_EXECUTABLE ? { binary: process.env.FIREFOX_EXECUTABLE } : {}),
      args: process.env.TENCHU_HEADLESS === "1" ? ["-headless"] : [],
      prefs: { "media.volume_scale": "0.0" },
    },
  } } });
  session = created.sessionId;
  console.log(`Firefox ${created.capabilities.browserVersion}: starting game`);
  const fullscreen = process.env.TENCHU_WEB_FULLSCREEN === "1";
  if (fullscreen) await command("POST", "/window/fullscreen", {});
  else await command("POST", "/window/rect", { width: 1280, height: 800 });
  const url = new URL(process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:8000");
  url.searchParams.set("renderer", "webgl2");
  await command("POST", "/url", { url: url.href });
  await run("window.testErrors = []; window.addEventListener('error', event => testErrors.push(event.message)); window.addEventListener('unhandledrejection', event => testErrors.push(String(event.reason)));");
  await run("window.gpuProbes = 0; Object.defineProperty(navigator, 'gpu', { value: { requestAdapter() { window.gpuProbes++; throw new Error('Explicit WebGL2 must not probe WebGPU'); } } });");
  await run(`const probe = document.createElement("script"); probe.textContent = ${JSON.stringify(`(${installFrameProbe.toString()})();`)}; document.head.append(probe);`);
  await run(`const probe = document.createElement("script"); probe.textContent = ${JSON.stringify(`(${installAudioProbe.toString()})();`)}; document.head.append(probe);`);
  if (process.env.TENCHU_WEB_INTERPOLATION === "1") {
    await run('localStorage.setItem("tenchu-pc/settings.json", JSON.stringify([...new TextEncoder().encode(JSON.stringify({graphics:{interpolation:true}}))]));');
  }
  await click("#quick");
  await wait("document.body.dataset.ready === 'true'");
  console.log("Game ready");
  await run("const script = document.createElement('script'); script.type = 'module'; script.textContent = \"import * as port from './pkg/tenchu_pc.js'; window.port = port;\"; document.head.append(script);");
  await wait("window.port != null");
  await wait("document.body.dataset.assetsReady === 'true'", 120000);
  console.log("Assets ready");
  if (fullscreen) {
    await click("#fullscreen");
    await wait("document.fullscreenElement?.id === 'game'");
  }
  await shot("launcher");
  await click("#game");
  await key("\uE006");
  assert.equal((await state()).screen, "equipment");
  await key("\uE006");
  await wait("Number(document.body.dataset.tick) > 10");
  const before = await state();
  await key("w", 1000); await key(" ");
  const after = await state();
  assert.equal(after.screen, "playing");
  assert(after.tick > before.tick);
  assert.notDeepEqual(after.position, before.position);
  assert.equal(after.fault, "");
  assert(await run("return window.audioProbe.nonSilent"), "The mixer must schedule audible samples");
  assert.equal(await run("return window.audioProbe.latencyReads"), 0, "Unused hardware latency must not be queried");
  await shot("mission-1", true);
  if (process.env.TENCHU_WEB_BENCH === "1") {
    for (const size of fullscreen ? [null] : [{ width: 1280, height: 800 }, { width: 3840, height: 2160 }]) {
      if (size) await command("POST", "/window/rect", size);
      await delay(5000);
      const timing = await command("POST", "/execute/async", {
        script: `(${measureFrames.toString()})().then(arguments[0]);`, args: [] });
      console.log(JSON.stringify({ scene: "playing", ...timing }));
      await key("\uE032"); // Free camera moves independently of the 30 Hz game.
      await command("POST", "/actions", { actions: [{ type: "key", id: "keyboard", actions: [{ type: "keyDown", value: "d" }] }] });
      const moving = await command("POST", "/execute/async", {
        script: `(${measureFrames.toString()})().then(arguments[0]);`, args: [] });
      await command("DELETE", "/actions");
      console.log(JSON.stringify({ scene: "moving-camera", ...moving }));
      await key("\uE032");
    }
    if (!fullscreen) await command("POST", "/window/rect", { width: 1280, height: 800 });
  }
  await key("\uE033");
  const paused = await state();
  const idle = await command("POST", "/execute/async", {
    script: `(${measureFrames.toString()})(750).then(arguments[0]);`, args: [] });
  assert.equal(idle.frames, 0, "A paused, unchanged scene must retain its image without rendering");
  const original = await shot("frozen-original", true);
  await key("\uE031"); await key("\uE004"); await key("\uE012");
  await shot("settings");
  await key("\uE031");
  const enhanced = await shot("surface-relief", true);
  assert.deepEqual(enhanced.hud, original.hud, "Post-processing must preserve the life medallion");
  assert.equal((await state()).tick, paused.tick);
  assert.equal((await state()).rng, paused.rng);
  await key("\uE033");
  assert((await state()).tick > paused.tick);
  assert.deepEqual(await run("return window.testErrors"), []);
  assert.equal(await run("return window.gpuProbes"), 0);
  assert.equal(await run("return document.body.dataset.renderer"), "WebGL2");
  console.log(JSON.stringify({ browser: created.capabilities.browserVersion,
    renderer: await run("return document.body.dataset.renderer"), before, after }, null, 2));
} catch (error) {
  await shot("failed").catch(() => {});
  console.error(log);
  throw error;
} finally {
  if (session) await command("DELETE", "").catch(() => {});
  driver.kill();
}
