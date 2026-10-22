// Runs in the browser against a screenshot, after the compositor has presented.
export async function sceneImage(base64) {
  const image = new Image();
  image.src = `data:image/png;base64,${base64}`;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = image.width;
  canvas.height = image.height;
  const context = canvas.getContext("2d");
  context.drawImage(image, 0, 0);
  const { data, width, height } = context.getImageData(0, 0, canvas.width, canvas.height);
  const colors = new Set();
  for (let y = Math.floor(height / 8); y < height * 7 / 8; y += 4) {
    for (let x = Math.floor(width / 8); x < width * 7 / 8; x += 4) {
      const i = (y * width + x) * 4;
      colors.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2]);
    }
  }
  // The opaque center of the life medallion must survive post-processing unchanged.
  const rect = document.getElementById("game").getBoundingClientRect();
  const scale = width / window.innerWidth;
  const hud = [];
  for (let y = 88; y <= 94; y += 2) {
    for (let x = -137; x <= -131; x += 2) {
      const px = Math.floor((rect.left + rect.width / 2 + x * rect.height / 240) * scale);
      const py = Math.floor((rect.top + rect.height / 2 + y * rect.height / 240) * scale);
      const i = (py * width + px) * 4;
      hud.push(...data.slice(i, i + 3));
    }
  }
  return { colors: colors.size, hud };
}
