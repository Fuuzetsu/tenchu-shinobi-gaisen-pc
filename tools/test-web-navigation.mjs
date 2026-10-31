import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { sceneImage } from "./browser-image.mjs";

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const base = process.env.TENCHU_WEB_URL ?? "http://127.0.0.1:44101";
const backend = process.env.TENCHU_WEBGL === "1" ? "webgl2" : "webgpu";
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/navigation-${backend}`;
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.BROWSER_EXECUTABLE || undefined,
  headless: false,
  args: ["--no-sandbox", "--mute-audio", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist",
    ...(process.platform === "linux" ? ["--ozone-platform=x11", "--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan,VulkanFromANGLE", "--disable-vulkan-surface"] : [])],
});
try {
  for (const character of [0, 1]) {
    const context = await browser.newContext({ viewport: { width: 1280, height: 760 } });
    await context.addInitScript(() => {
      const settings = { pause_on_defocus: false, sharp_text: true, navigation: { camera_relative: true },
        graphics: { interpolation: true, fog: false } };
      localStorage.setItem("tenchu-pc/settings.json", JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
      const pad = { id: "Navigation test controller", index: 0, mapping: "standard", connected: false,
        axes: [0, 0, 0, 0], buttons: Array.from({ length: 17 }, () => ({pressed:false,touched:false,value:0})),
        get timestamp() { return performance.now(); } };
      window.navigationPad = pad;
      Object.defineProperty(navigator, "getGamepads", { value: () => [pad.connected ? pad : null] });
      window.connectNavigationPad = () => {
        pad.connected = true;
        const event = new Event("gamepadconnected");
        Object.defineProperty(event, "gamepad", { value: pad });
        window.dispatchEvent(event);
      };
    });
    const page = await context.newPage(), errors = [];
    page.on("pageerror", e => errors.push(String(e)));
    page.on("console", e => { if(e.type()==="error" || /GL_INVALID_|WebGL: INVALID_/.test(e.text())) errors.push(e.text()); });
    const state = () => page.evaluate(() => JSON.parse(window.port.snapshot()));
    const ticks = async count => {
      const start = (await state()).tick;
      await page.waitForFunction(t => JSON.parse(window.port.snapshot()).tick >= t, start + count, { timeout: 15000 });
    };
    const key = async name => { await page.keyboard.press(name); await page.waitForTimeout(160); };
    const shot = async name => {
      const png = await page.screenshot({path:`${output}/${character}-${name}.png`});
      assert((await page.evaluate(sceneImage,png.toString("base64"))).colors > 64);
    };
    try {
      await page.goto(`${base}/?renderer=${backend}`);
      await page.click("#quick");
      await page.waitForFunction(()=>document.body.dataset.ready==="true", null, {timeout:90000});
      await page.evaluate(async()=>{window.port=await import("./pkg/tenchu_pc.js");});
      await page.waitForFunction(()=>document.body.dataset.assetsReady==="true", null, {timeout:180000});
      await page.locator("#game").click();
      if(character) { await key("ArrowDown");await key("ArrowRight");await key("ArrowUp"); }
      await key("Enter");
      await page.waitForFunction(()=>JSON.parse(window.port.snapshot()).screen==="equipment",null,{timeout:90000});
      await page.waitForTimeout(500); await key("Space"); await ticks(35);
      assert.equal((await state()).character,character);
      let before=await state();
      await page.keyboard.down("a"); await ticks(8); await page.keyboard.up("a"); await ticks(3);
      let after=await state();
      assert.notDeepEqual(after.position,before.position,"Left must move, not just turn");
      await shot("camera-relative-left");
      await page.mouse.move(600,380); await page.mouse.down({button:"right"});
      await page.waitForFunction(()=>document.pointerLockElement!==null);
      before=await state();
      await page.mouse.move(950,430,{steps:12}); await ticks(3);
      await page.mouse.up({button:"right"});
      await page.waitForFunction(()=>document.pointerLockElement===null);
      after=await state();
      const direction=s=>s.camera_target.map((v,i)=>v-s.camera_position[i]);
      assert.notDeepEqual(direction(after),direction(before),"Held right mouse must orbit");
      await shot("mouse-orbit");
      await page.evaluate(()=>window.connectNavigationPad()); await ticks(3);
      before=await state();
      await page.evaluate(()=>{window.navigationPad.axes=[0.37,-0.81,0,0];});
      await ticks(8);
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0,0];}); await ticks(3);
      after=await state();
      assert.notDeepEqual(after.position,before.position,"Analog stick must move the character");
      const view=direction(before), viewLength=Math.hypot(view[0],view[2]);
      const expected=[(view[0]*0.81+view[2]*0.37)/viewLength,(view[2]*0.81-view[0]*0.37)/viewLength];
      const moved=[after.position[0]-before.position[0],after.position[2]-before.position[2]];
      assert((moved[0]*expected[0]+moved[1]*expected[1])/Math.hypot(...moved)/Math.hypot(...expected)>0.98,
        "The physical stick angle must survive browser/controller mapping");
      before=await state();
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0.72,0.2];}); await ticks(15);
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0,0];}); await ticks(2);
      after=await state();
      assert.notDeepEqual(direction(after),direction(before),"Right stick must orbit");
      await shot("controller-orbit");
      await page.evaluate(()=>{window.navigationPad.buttons[11]={pressed:true,touched:true,value:1};}); await ticks(2);
      await page.evaluate(()=>{window.navigationPad.buttons[11]={pressed:false,touched:false,value:0};}); await ticks(2);
      before=await state();
      const facing=before.yaw*Math.PI*2/4096, centered=direction(before);
      assert((centered[0]*-Math.sin(facing)+centered[2]*-Math.cos(facing))/Math.hypot(centered[0],centered[2])>0.99,
        "Clicking the right stick recenters behind the player");
      await page.keyboard.down("ControlLeft"); await page.keyboard.down("s"); await ticks(7);
      await page.keyboard.up("s"); await page.keyboard.up("ControlLeft"); await ticks(3);
      after=await state();
      assert.equal(after.yaw,before.yaw,"The classic modifier must allow backing away without turning");
      assert.notDeepEqual(after.position,before.position);
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0.72,-0.2];}); await ticks(15);
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0,0];}); await ticks(2);
      const aimView=direction(await state());
      const aimYaw=Math.atan2(-aimView[0],-aimView[2])*4096/(Math.PI*2);
      await page.keyboard.down("e"); await ticks(12); before=await state();
      assert.equal(before.motion,0x400,"Holding item use enters grapple aim");
      const yawError=((before.yaw-aimYaw+6144)%4096)-2048;
      assert(Math.abs(yawError)<3,"Starting grapple must face the camera's heading");
      await page.keyboard.down("d"); await ticks(6); await page.keyboard.up("d");
      after=await state();
      assert.deepEqual(after.position,before.position,"Grapple aiming must not walk the character");
      assert.notDeepEqual(direction(after),direction(before),"Grapple retains directional aim");
      before=after;
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,-1,0];}); await ticks(6);
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0,0];});
      after=await state();
      assert.notDeepEqual(direction(after),direction(before),"The right stick also aims items");
      await shot("grapple-aim");
      await key("ShiftLeft"); await page.keyboard.up("e"); await ticks(15);
      await page.mouse.click(600,380,{button:"middle"}); await ticks(3);
      before=await state();
      const recentered=direction(before), heroAngle=before.yaw*Math.PI*2/4096;
      assert((recentered[0]*-Math.sin(heroAngle)+recentered[2]*-Math.cos(heroAngle))/Math.hypot(recentered[0],recentered[2])>0.99);
      await key("F1"); await page.keyboard.press("Shift+Tab"); await page.waitForTimeout(300);
      await shot("movement-settings");
      const frozen=await state();
      await page.evaluate(()=>{window.navigationPad.axes=[0.4,0,0.4,0];}); await page.waitForTimeout(300);
      assert.deepEqual(await state(),frozen,"The open settings menu must pause movement and camera");
      await page.evaluate(()=>{window.navigationPad.axes=[0,0,0,0];});
      await key("Enter"); await key("F1"); await ticks(12);
      before=await state();
      await page.keyboard.down("a"); await ticks(8); await page.keyboard.up("a"); await ticks(2);
      after=await state();
      assert.notEqual(after.yaw,before.yaw,"Classic left must still turn");
      assert(Math.hypot(after.position[0]-before.position[0],after.position[2]-before.position[2])<100,"Classic turning must not walk sideways");
      assert.equal(after.fault,""); assert.deepEqual(errors,[]);
      await writeFile(`${output}/${character}-result.json`,JSON.stringify(after,null,2));
      console.log(`${backend}: hero ${character}, keyboard, mouse orbit, full analog movement/camera, menu and classic controls passed`);
    } catch(error) {
      await page.screenshot({path:`${output}/${character}-failure.png`}).catch(()=>{});
      await writeFile(`${output}/${character}-failure.json`,JSON.stringify({error:String(error),errors,state:await state().catch(()=>null)},null,2));
      throw error;
    } finally { await context.close(); }
  }
} finally { await browser.close(); }
