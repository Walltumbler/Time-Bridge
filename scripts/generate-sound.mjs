import { writeFileSync } from 'node:fs';
const rate=44100, seconds=1.4;
const pcm=Buffer.alloc(Math.floor(rate*seconds)*2);
for(let i=0;i<pcm.length/2;i++) {
  const t=i/rate, part=t%0.45;
  const envelope=part<0.32 ? Math.min(1,part/0.02)*Math.min(1,(0.32-part)/0.08) : 0;
  const value=(Math.sin(2*Math.PI*880*t)+0.35*Math.sin(2*Math.PI*1320*t))*envelope*0.32;
  pcm.writeInt16LE(Math.round(value*32767),i*2);
}
const h=Buffer.alloc(44);h.write('RIFF');h.writeUInt32LE(36+pcm.length,4);h.write('WAVEfmt ',8);h.writeUInt32LE(16,16);h.writeUInt16LE(1,20);h.writeUInt16LE(1,22);h.writeUInt32LE(rate,24);h.writeUInt32LE(rate*2,28);h.writeUInt16LE(2,32);h.writeUInt16LE(16,34);h.write('data',36);h.writeUInt32LE(pcm.length,40);
writeFileSync('src-tauri/assets/alert.wav',Buffer.concat([h,pcm]));
