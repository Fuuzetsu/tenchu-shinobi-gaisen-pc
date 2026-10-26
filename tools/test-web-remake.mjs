import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { installFrameProbe, measureFrames } from './browser-frames.mjs';
import { sceneImage } from './browser-image.mjs';
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
const backend = process.env.TENCHU_WEBGL === '1' ? 'webgl2' : 'webgpu';
const output = process.env.TENCHU_WEB_CAPTURES ?? `private/remake-review/${backend}`;
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.BROWSER_EXECUTABLE,
  headless: process.env.TENCHU_HEADLESS === '1', args: ['--no-sandbox','--mute-audio','--enable-unsafe-webgpu','--ignore-gpu-blocklist','--ozone-platform=x11','--enable-gpu','--use-angle=vulkan','--enable-features=Vulkan,VulkanFromANGLE','--disable-vulkan-surface'] });
const context = await browser.newContext({ viewport: { width:1280,height:760 } });
await context.addInitScript(() => {
  const settings={ pause_on_defocus:false, sharp_text:true, graphics:{ weather:{mode:'Snow',snow:100,amount:100} } };
  localStorage.setItem('tenchu-pc/settings.json',JSON.stringify(Array.from(new TextEncoder().encode(JSON.stringify(settings)))));
  window.gpuErrors=[];
  if(typeof GPUAdapter !== 'undefined') { const request=GPUAdapter.prototype.requestDevice; GPUAdapter.prototype.requestDevice=async function(...args) { const device=await request.apply(this,args); device.addEventListener('uncapturederror',e=>window.gpuErrors.push(e.error.message)); return device; }; }
});
await context.addInitScript(installFrameProbe);
const page=await context.newPage(),errors=[];
page.on('pageerror',e=>errors.push(String(e)));
page.on('console',m=>{if(m.type()==='error'||/GL_INVALID_|WebGL: INVALID_/.test(m.text()))errors.push(m.text());});
const state=()=>page.evaluate(()=>JSON.parse(window.port.snapshot()));
const settings=()=>page.evaluate(()=>JSON.parse(new TextDecoder().decode(new Uint8Array(JSON.parse(localStorage.getItem('tenchu-pc/settings.json'))))));
const key=async name=>{await page.keyboard.press(name);await page.waitForTimeout(200);};
let category=0,row=0;
const tab=async target=>{while(category!==target){await key('Tab');category=(category+1)%19;}row=0;};
const select=async target=>{while(row!==target){await key(row<target?'ArrowDown':'ArrowUp');row+=row<target?1:-1;}};
const shot=async name=>{const png=await page.screenshot({path:`${output}/${name}.png`}); assert((await page.evaluate(sceneImage,png.toString('base64'))).colors>64,`${name}: blank scene`); if(backend==='webgl2')assert.equal(await page.evaluate(()=>document.getElementById('game').getContext('webgl2').getError()),0);};
try {
 await page.goto(`${process.env.TENCHU_WEB_URL??'http://127.0.0.1:44120'}/?renderer=${backend}`);
 await page.click('#quick');await page.waitForFunction(()=>document.body.dataset.ready==='true',null,{timeout:120000});
 await page.evaluate(async()=>{window.port=await import('./pkg/tenchu_pc.js');});
 await page.waitForFunction(()=>document.body.dataset.assetsReady==='true',null,{timeout:180000});
 await page.locator('#game').click();await page.mouse.move(5,5);await key('Enter');await key('Enter');
 await page.waitForFunction(()=>Number(document.body.dataset.tick)>=60,null,{timeout:90000});
 await key('F1'); const frozen=await state();
 await select(1);await key('ArrowLeft');await key('ArrowLeft');await key('ArrowLeft');
 let high=await settings();assert.equal(high.graphics.models.foliage,3);assert.equal(high.graphics.fog,false);assert.equal(high.graphics.blob_shadows,false);assert.equal(high.graphics.samples,2);assert.equal(high.graphics.effects.leaves,true);
 await shot('modern-high');assert.deepEqual(await state(),frozen);
 await select(0);await key('ArrowRight');
 const living=await settings();assert.equal(living.keep_enemies_active,true);assert.equal(living.sight_percent,100);assert.deepEqual(living.graphics,high.graphics);
 await key('ArrowRight');assert.equal((await settings()).sight_percent,125);
 high=await settings();await key('F8');assert.equal((await settings()).retro,true);await shot('retro');assert.deepEqual((await settings()).graphics,high.graphics);assert.equal((await settings()).sight_percent,125);assert.deepEqual(await state(),frozen);
 await key('F8');assert.equal((await settings()).retro,false);assert.deepEqual((await settings()).graphics,high.graphics);await shot('restored');
 await key('Shift+Tab');category=18;row=0;await shot('contact-menu');await select(2);await key('ArrowRight');await key('r');assert.equal((await settings()).graphics.effects.spark_density,100);
 await key('F1');await page.waitForTimeout(5000);const highFrames=await page.evaluate(measureFrames,5000);
 await page.keyboard.down('w');await page.waitForTimeout(700);await page.keyboard.up('w');await key('F1');
 await tab(13);await select(1);const beforeClear=await state();await shot('snow-before-clear');await key('Enter');assert.equal((await settings()).graphics.weather.mode,'Clear');await shot('cleared');assert.deepEqual(await state(),beforeClear);
 await key('F1');await page.waitForTimeout(3000);const clearFrames=await page.evaluate(measureFrames,5000);await key('F1');
 await tab(0);await select(1);await key('ArrowRight');
 const ultra=await settings();assert.equal(ultra.graphics.models.foliage,4);assert.equal(ultra.graphics.samples,4);await shot('modern-ultra');
 await tab(2);await select(1);await key('ArrowRight');await shot('clipped-sight');
 assert.equal((await state()).fault,'');assert.deepEqual(await page.evaluate(()=>window.gpuErrors),[]);assert.deepEqual(errors,[]);
 await writeFile(`${output}/result.json`,JSON.stringify({backend,highFrames,clearFrames,state:await state(),high,ultra,errors},null,2));
 console.log(`${backend}: High/Ultra, independent gameplay presets, F8 roundtrip, contact settings, Shift+Tab/R and clear weather passed`);
} catch(error) {await page.screenshot({path:`${output}/failure.png`}).catch(()=>{});await writeFile(`${output}/failure.json`,JSON.stringify({error:String(error),errors,state:await state().catch(()=>null)},null,2));throw error;}
finally {await context.close();await browser.close();}
