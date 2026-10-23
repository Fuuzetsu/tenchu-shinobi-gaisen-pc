const cached = new Map();
let currentMovie, paused = false, movieGain = 1, movieMusic = 1, stereo = true, decoder;
const canvas = document.createElement("canvas");
canvas.width = 320;
canvas.height = 240;
const drawing = canvas.getContext("2d", { willReadFrequently: true });

export function graphics_backend(name, adapter) {
  document.body.dataset.renderer = name;
  document.body.dataset.adapter = adapter;
  const label = document.getElementById("graphics-info");
  label.textContent = name;
  label.title = adapter;
}

export function graphics_preference() { return document.getElementById("renderer").value; }

function media(path) {
  if (!cached.has(path)) {
    const request = fetch(path).then(response => {
      if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
      return response.blob();
    }).catch(error => { cached.delete(path); throw error; });
    cached.set(path, request);
  }
  return cached.get(path);
}

export async function fetch_asset(path, compressed) {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  if (!compressed) return response.arrayBuffer();
  return new Response(response.body.pipeThrough(new DecompressionStream("gzip"))).arrayBuffer();
}

export async function fetch_artwork(paths) {
  return Promise.all(paths.map(path =>
    fetch_asset(`data/pack/${path.split("/").map(encodeURIComponent).join("/")}`, false)));
}

export async function prefetch_media(paths) {
  let next = 0, complete = 0;
  await Promise.all(Array.from({ length: Math.min(6, paths.length) }, async () => {
    while (next < paths.length) {
      await media(paths[next++]);
      status(`Preparing media in the background: ${++complete}/${paths.length}`);
    }
  }));
}

export function status(message) { document.getElementById("status").textContent = message; }
export function ready() {
  document.body.dataset.ready = "true";
  document.getElementById("welcome").hidden = true;
  document.getElementById("game").focus();
}
export function error(message) {
  document.getElementById("retry").hidden = false;
  document.getElementById("welcome").hidden = false;
  document.getElementById("message").textContent = message;
  document.body.dataset.error = message;
  console.error(message);
}
export function media_pause(value) {
  paused = value;
  if (!currentMovie) return;
  if (paused) currentMovie.pause();
  else if (currentMovie.readyState >= 2) play_movie(currentMovie);
}
export function movie_volume(master, music) {
  if (music !== 255) movieMusic = music / 96;
  movieGain = master / 100 * movieMusic;
  if (currentMovie) currentMovie.gain.gain.value = movieGain;
}
export function movie_stereo(value) {
  stereo = value;
  if (currentMovie) currentMovie.gain.channelCount = stereo ? 2 : 1;
}
function audio_context() { return decoder ??= new AudioContext({ sampleRate: 37800 }); }
export function configure_game_audio(context) {
  // CPAL reads this on every buffer for timestamps our mixer ignores. Firefox's
  // synchronous audio-device IPC can deadlock here. Scheduling uses currentTime.
  Object.defineProperty(context, "outputLatency", { value: 0 });
}
function play_movie(video) {
  audio_context().resume().catch(cause => { video.dataset.error = String(cause); });
  video.play().catch(cause => { if (video.dataset.active === "true" && cause.name !== "AbortError") video.dataset.error = String(cause); });
}
export function movie_error(video) { return video.dataset.error || video.error?.message || ""; }

export async function audio_clip(path) {
  const sound = await audio_context().decodeAudioData(await (await media(`data/${path}`)).arrayBuffer());
  return { rate: sound.sampleRate, left: sound.getChannelData(0), right: sound.getChannelData(Math.min(1, sound.numberOfChannels - 1)) };
}
export function movie_create(path) {
  const video = document.createElement("video");
  video.playsInline = true;
  const context = audio_context();
  video.source = context.createMediaElementSource(video);
  video.gain = context.createGain();
  video.gain.gain.value = movieGain;
  video.gain.channelCountMode = "explicit";
  video.gain.channelCount = stereo ? 2 : 1;
  video.source.connect(video.gain).connect(context.destination);
  video.dataset.active = "true";
  currentMovie = video;
  media(`data/${path}`).then(blob => {
    if (video.dataset.active !== "true") return;
    video.src = URL.createObjectURL(blob);
    if (!paused) play_movie(video);
  }).catch(cause => { video.dataset.error = String(cause); });
  return video;
}
export function movie_pixels(video) {
  drawing.drawImage(video, 0, 0, 320, 240);
  return drawing.getImageData(0, 0, 320, 240).data;
}
export function movie_close(video) {
  video.dataset.active = "false";
  video.pause();
  if (video.src) URL.revokeObjectURL(video.src);
  video.removeAttribute("src");
  video.load();
  video.source.disconnect();
  video.gain.disconnect();
  if (currentMovie === video) currentMovie = undefined;
}

export function frame(tick, playing, fault) {
  document.body.dataset.tick = String(tick);
  document.body.dataset.playing = String(playing);
  document.body.dataset.fault = fault;
}

export function loading_state(ready) { document.body.dataset.assetsReady = String(ready); }
export function closed() {
  document.body.dataset.ready = "false";
  document.getElementById("welcome").hidden = false;
  document.getElementById("message").textContent = "Game closed. Your progress is saved in this browser.";
  document.querySelectorAll(".actions button").forEach(button => { button.disabled = false; });
}

export function download_frame() {
  document.getElementById("game").toBlob(blob => {
    if (!blob) { status("Screenshot capture failed"); return; }
    const url = URL.createObjectURL(blob), link = document.createElement("a");
    link.href = url;
    link.download = `tenchu-${new Date().toISOString().replaceAll(":", "-")}.png`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }, "image/png");
}
