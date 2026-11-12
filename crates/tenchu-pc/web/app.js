import init, { play, validate_save, validate_campaign_index } from "./pkg/tenchu_pc.js";
let initialized;
document.getElementById("game").addEventListener("contextmenu", event => event.preventDefault());
const renderer = document.getElementById("renderer");
const preferred = new URL(location.href).searchParams.get("renderer") ?? localStorage.getItem("tenchu-pc/renderer");
if (["auto", "webgl2", "webgpu"].includes(preferred)) renderer.value = preferred;
renderer.onchange = () => {
  localStorage.setItem("tenchu-pc/renderer", renderer.value);
  const url = new URL(location.href);
  if (url.searchParams.has("renderer")) {
    url.searchParams.set("renderer", renderer.value);
    history.replaceState(null, "", url);
  }
  if (document.body.dataset.renderer) location.reload();
};
const start = async quick => {
  const buttons = [document.getElementById("play"), document.getElementById("quick")];
  buttons.forEach(button => { button.disabled = true; });
  try {
    if (!window.isSecureContext) throw new Error("Open this game on localhost or HTTPS. Browser audio and gamepad support require a secure connection.");
    initialized ??= init();
    await initialized;
    await play(quick);
  } catch (error) {
    document.getElementById("retry").hidden = false;
    document.getElementById("message").textContent = error.message ?? error;
    document.body.dataset.error = String(error);
    buttons.forEach(button => { button.disabled = false; });
    console.error(error);
  }
};
document.getElementById("play").onclick = () => start(false);
document.getElementById("retry").onclick = () => location.reload();
document.getElementById("quick").onclick = () => start(true);
document.getElementById("fullscreen").onclick = () => {
  document.getElementById("game").focus();
  const action = document.fullscreenElement ? document.exitFullscreen() : document.getElementById("game").requestFullscreen();
  action.catch(error => { document.getElementById("status").textContent = error.message; });
};
document.addEventListener("fullscreenchange", () => {
  if (document.body.dataset.ready === "true") document.getElementById("game").focus();
});
const campaignPrefix = "tenchu-pc/progress.sav.campaigns/";
function saveBytes(text) {
  if (text == null) return new Uint8Array();
  const bytes = JSON.parse(text);
  if (!Array.isArray(bytes) || !bytes.every(n => Number.isInteger(n) && n >= 0 && n <= 255)) throw new Error("Invalid save data");
  return new Uint8Array(bytes);
}

document.getElementById("export").onclick = async () => {
  try {
    initialized ??= init();
    await initialized;
    const campaigns = {};
    const index = localStorage.getItem(`${campaignPrefix}index.json`);
    if (index != null) {
      campaigns["index.json"] = index;
      for (const name of JSON.parse(validate_campaign_index(saveBytes(index)))) {
        campaigns[name] = localStorage.getItem(`${campaignPrefix}${name}`);
      }
    }
    const data = { version: 2, progress: localStorage.getItem("tenchu-pc/progress.sav"), settings: localStorage.getItem("tenchu-pc/settings.json"), campaigns };
    const url = URL.createObjectURL(new Blob([JSON.stringify(data)], { type: "application/json" }));
    const link = document.createElement("a");
    link.href = url; link.download = "tenchu-save.json"; link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  } catch (error) { document.getElementById("status").textContent = `Export failed: ${error.message}`; }
};
document.getElementById("import").onclick = () => document.getElementById("save-file").click();
document.getElementById("save-file").onchange = async event => {
  try {
    const file = event.target.files[0];
    if (!file) return;
    if (file.size > 16 * 1024 * 1024) throw new Error("Save file is too large");
    const data = JSON.parse(await file.text());
    if (data.version !== 1 && data.version !== 2) throw new Error("Unsupported save file");
    initialized ??= init();
    await initialized;
    validate_save(saveBytes(data.progress), saveBytes(data.settings));
    const changes = new Map();
    for (const [key, name] of [["progress", "progress.sav"], ["settings", "settings.json"]]) {
      if (data[key] != null || data.version === 2) changes.set(`tenchu-pc/${name}`, data[key] ?? null);
    }
    if (data.version === 2) {
      const campaigns = data.campaigns;
      if (!campaigns || typeof campaigns !== "object" || Array.isArray(campaigns)) throw new Error("Invalid campaign collection");
      const names = campaigns["index.json"] == null ? [] : ["index.json", ...JSON.parse(validate_campaign_index(saveBytes(campaigns["index.json"])))];
      if (Object.keys(campaigns).length !== names.length || names.some(name => !Object.hasOwn(campaigns, name))) throw new Error("Incomplete campaign collection");
      for (const name of names) {
        if (name !== "index.json") validate_save(saveBytes(campaigns[name]), new Uint8Array());
      }
      for (const key of Object.keys(localStorage)) {
        if (key.startsWith(campaignPrefix)) changes.set(key, null);
      }
      // Write the index last, after every referenced save has been restored.
      for (const name of names.filter(name => name !== "index.json")) changes.set(`${campaignPrefix}${name}`, campaigns[name]);
      changes.delete(`${campaignPrefix}index.json`);
      changes.set(`${campaignPrefix}index.json`, campaigns["index.json"] ?? null);
    } else if (data.progress != null) {
      const text = localStorage.getItem(`${campaignPrefix}index.json`);
      if (text != null) {
        validate_campaign_index(saveBytes(text));
        const index = JSON.parse(new TextDecoder().decode(saveBytes(text)));
        index.active = 0;
        changes.set(`${campaignPrefix}index.json`, JSON.stringify([...new TextEncoder().encode(JSON.stringify(index))]));
      }
    }
    const previous = new Map([...changes.keys()].map(key => [key, localStorage.getItem(key)]));
    const put = (key, value) => value == null ? localStorage.removeItem(key) : localStorage.setItem(key, value);
    try {
      for (const [key, value] of changes) put(key, value);
    } catch (error) {
      for (const key of changes.keys()) localStorage.removeItem(key);
      for (const [key, value] of previous) put(key, value);
      throw error;
    }
    location.reload();
  } catch (error) { document.getElementById("status").textContent = `Import failed: ${error.message}`; }
};
