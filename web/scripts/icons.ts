import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import pngToIco from 'png-to-ico';
import sharp from 'sharp';

const SOURCE = new URL('../assets/mascot.png', import.meta.url).pathname;
const STATIC_DIR = new URL('../static/', import.meta.url).pathname;

if (!existsSync(SOURCE)) {
	console.error(`missing source image: ${SOURCE}`);
	process.exit(1);
}

await mkdir(STATIC_DIR, { recursive: true });

const outputs: Array<[string, Promise<Buffer>]> = [
	[
		'mascot.webp',
		sharp(SOURCE).resize(1024, 1024, { fit: 'inside' }).webp({ quality: 80 }).toBuffer()
	],
	['favicon-32.png', sharp(SOURCE).resize(32, 32).png().toBuffer()],
	['favicon-192.png', sharp(SOURCE).resize(192, 192).png().toBuffer()],
	[
		'apple-touch-icon.png',
		sharp(SOURCE)
			.resize(180, 180, { fit: 'contain', background: { r: 251, g: 253, b: 255, alpha: 1 } })
			.flatten({ background: { r: 251, g: 253, b: 255 } })
			.png()
			.toBuffer()
	]
];

for (const [name, pending] of outputs) {
	await writeFile(`${STATIC_DIR}${name}`, await pending);
	console.log(`wrote static/${name}`);
}

const ico = await pngToIco(
	await sharp(SOURCE).resize(48, 48).png({ compressionLevel: 9, palette: true }).toBuffer()
);
await writeFile(`${STATIC_DIR}favicon.ico`, ico);
console.log('wrote static/favicon.ico');
