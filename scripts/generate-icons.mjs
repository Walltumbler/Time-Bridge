// Deterministic app artwork. Uses only Node's standard library.
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
const size = 256;
const raw = Buffer.alloc((size * 4 + 1) * size);
for (let y = 0; y < size; y++)
  for (let x = 0; x < size; x++) {
    const p = y * (size * 4 + 1) + 1 + x * 4;
    const rounded =
      (x < 36 && y < 36 && (x - 36) ** 2 + (y - 36) ** 2 > 36 ** 2) ||
      (x > 219 && y < 36 && (x - 219) ** 2 + (y - 36) ** 2 > 36 ** 2) ||
      (x < 36 && y > 219 && (x - 36) ** 2 + (y - 219) ** 2 > 36 ** 2) ||
      (x > 219 && y > 219 && (x - 219) ** 2 + (y - 219) ** 2 > 36 ** 2);
    const arch =
      Math.abs(y - (74 + 0.009 * (x - 128) ** 2)) < 7 && x > 42 && x < 214;
    const pillar =
      [48, 88, 168, 208].some((n) => Math.abs(x - n) < 6) && y > 112 && y < 186;
    const deck = y > 179 && y < 191 && x > 33 && x < 223;
    const hand =
      (Math.abs(x - 128) < 5 && y > 98 && y < 140) ||
      (y > 133 && y < 143 && x > 124 && x < 151);
    const color =
      arch || pillar || deck
        ? [222, 238, 250]
        : hand
          ? [95, 176, 255]
          : [21, 43, 60];
    raw[p] = color[0];
    raw[p + 1] = color[1];
    raw[p + 2] = color[2];
    raw[p + 3] = rounded ? 0 : 255;
  }
function crc(buf) {
  let c = 0xffffffff;
  for (const b of buf) {
    c ^= b;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const t = Buffer.from(type),
    s = Buffer.alloc(4),
    c = Buffer.alloc(4);
  s.writeUInt32BE(data.length);
  c.writeUInt32BE(crc(Buffer.concat([t, data])));
  return Buffer.concat([s, t, data, c]);
}
const header = Buffer.alloc(13);
header.writeUInt32BE(size, 0);
header.writeUInt32BE(size, 4);
header[8] = 8;
header[9] = 6;
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk("IHDR", header),
  chunk("IDAT", deflateSync(raw)),
  chunk("IEND", Buffer.alloc(0)),
]);
mkdirSync("src-tauri/icons", { recursive: true });
writeFileSync("src-tauri/icons/icon.png", png);
const ico = Buffer.alloc(22);
ico.writeUInt16LE(1, 2);
ico.writeUInt16LE(1, 4);
ico.writeUInt16LE(1, 10);
ico.writeUInt16LE(32, 12);
ico.writeUInt32LE(png.length, 14);
ico.writeUInt32LE(22, 18);
writeFileSync("src-tauri/icons/icon.ico", Buffer.concat([ico, png]));
