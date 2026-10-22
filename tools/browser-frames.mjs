// Install before starting the game. Browser callbacks can run without presenting a frame.
export function installFrameProbe() {
  const probe = window.frameProbe = { presents: 0, active: false, finish: false, samples: [] };
  const blit = WebGL2RenderingContext.prototype.blitFramebuffer;
  WebGL2RenderingContext.prototype.blitFramebuffer = function (...args) {
    const result = blit.apply(this, args);
    if (this.canvas.id === "game") { probe.presents++; probe.gl = this; }
    return result;
  };
  if (typeof GPUCanvasContext !== "undefined") {
    const texture = GPUCanvasContext.prototype.getCurrentTexture;
    GPUCanvasContext.prototype.getCurrentTexture = function (...args) {
      const result = texture.apply(this, args);
      if (this.canvas.id === "game") probe.presents++;
      return result;
    };
  }
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = callback => raf(now => {
    if (!probe.active) { callback(now); return; }
    const count = probe.presents, start = performance.now();
    callback(now);
    const submitted = performance.now();
    if (probe.presents !== count) {
      if (probe.finish && probe.gl) probe.gl.finish();
      probe.samples.push({ cpu: submitted - start, wait: performance.now() - submitted });
    }
  });
}

export async function measureFrames(milliseconds = 5000, finish = false) {
  const probe = window.frameProbe, canvas = document.getElementById("game");
  const count = probe.presents, start = performance.now();
  const tick = JSON.parse(window.port.snapshot()).tick;
  let callbacks = 0, previous, stopped = false;
  const intervals = [];
  probe.samples = []; probe.finish = finish; probe.active = true;
  const callback = now => {
    if (stopped) return;
    if (previous != null) intervals.push(now - previous);
    previous = now; callbacks++;
    requestAnimationFrame(callback);
  };
  requestAnimationFrame(callback);
  await new Promise(resolve => setTimeout(resolve, milliseconds));
  const elapsed = performance.now() - start;
  stopped = true; probe.active = false;
  const summary = values => {
    if (!values.length) return null;
    values.sort((a, b) => a - b);
    return { mean: values.reduce((a, b) => a + b, 0) / values.length,
      p95: values[Math.floor(values.length * .95)] };
  };
  const debug = probe.gl?.getExtension("WEBGL_debug_renderer_info");
  return { size: [canvas.width, canvas.height], elapsed, frames: probe.presents - count,
    renderedFps: (probe.presents - count) * 1000 / elapsed, callbacksPerSecond: callbacks * 1000 / elapsed,
    ticksPerSecond: (JSON.parse(window.port.snapshot()).tick - tick) * 1000 / elapsed,
    callbackIntervalMs: summary(intervals), cpuMs: summary(probe.samples.map(s => s.cpu)),
    finishWaitMs: finish ? summary(probe.samples.map(s => s.wait)) : null,
    renderer: debug ? probe.gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : document.body.dataset.renderer };
}
