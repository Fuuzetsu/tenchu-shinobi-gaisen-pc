import init, { play, validate_save } from "./pkg/tenchu_pc.js";
let initialized;
const start = async quick => {
  const buttons = [document.getElementById("play"), document.getElementById("quick")];
  buttons.forEach(button => { button.disabled = true; });
  try {
    if (!navigator.gpu) throw new Error("This browser needs WebGPU. Use a browser with WebGPU enabled on localhost or HTTPS.");
    initialized ??= init();
    await initialized;
    await play(quick);
  } catch (error) {
    document.getElementById("message").textContent = error.message ?? error;
    document.body.dataset.error = String(error);
    buttons.forEach(button => { button.disabled = false; });
    console.error(error);
  }
};
document.getElementById("play").onclick = () => start(false);
document.getElementById("quick").onclick = () => start(true);
document.getElementById("fullscreen").onclick = () => document.fullscreenElement ? document.exitFullscreen() : document.getElementById("game").requestFullscreen();
document.getElementById("export").onclick = () => {
  const data = { version: 1, progress: localStorage.getItem("tenchu-pc/progress.sav"), settings: localStorage.getItem("tenchu-pc/settings.json") };
  const url = URL.createObjectURL(new Blob([JSON.stringify(data)], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url; link.download = "tenchu-save.json"; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
};
document.getElementById("import").onclick = () => document.getElementById("save-file").click();
document.getElementById("save-file").onchange = async event => {
  try {
    const file = event.target.files[0];
    if (!file) return;
    if (file.size > 1024 * 1024) throw new Error("Save file is too large");
    const data = JSON.parse(await file.text());
    if (data.version !== 1) throw new Error("Unsupported save file");
    for (const key of ["progress", "settings"]) {
      if (data[key] != null) {
        const bytes = JSON.parse(data[key]);
        if (!Array.isArray(bytes) || !bytes.every(n => Number.isInteger(n) && n >= 0 && n <= 255)) throw new Error("Invalid save data");
      }
    }
    initialized ??= init();
    await initialized;
    validate_save(new Uint8Array(data.progress == null ? [] : JSON.parse(data.progress)), new Uint8Array(data.settings == null ? [] : JSON.parse(data.settings)));
    for (const [key, name] of [["progress", "progress.sav"], ["settings", "settings.json"]]) {
      if (data[key] != null) localStorage.setItem(`tenchu-pc/${name}`, data[key]);
    }
    location.reload();
  } catch (error) { document.getElementById("status").textContent = `Import failed: ${error.message}`; }
};
