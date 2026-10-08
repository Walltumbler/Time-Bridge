/** Resize a user-selected image to a bounded PNG suitable for an alarm. Browser only. */
export async function iconFromFile(file: Blob): Promise<string> {
  if (typeof document === "undefined") throw new Error("iconFromFile requires a browser. Native apps can provide a PNG data URL directly.");
  if (file.size > 5 * 1024 * 1024) throw new Error("Choose an image under 5 MB.");
  const bitmap = await createImageBitmap(file);
  try {
    const canvas = document.createElement("canvas"); canvas.width = canvas.height = 64;
    const context = canvas.getContext("2d"); if (!context) throw new Error("Cannot prepare this image.");
    const scale = Math.min(64 / bitmap.width, 64 / bitmap.height);
    const width = bitmap.width * scale, height = bitmap.height * scale;
    context.drawImage(bitmap, (64 - width) / 2, (64 - height) / 2, width, height);
    const icon = canvas.toDataURL("image/png");
    if (icon.length > 22000) throw new Error("Choose a simpler image.");
    return icon;
  } finally { bitmap.close(); }
}
