import captureWebsite from "capture-website";
import { $ } from "bun";
import { join } from "node:path";

const geoCardMargin = ".geocard { margin-bottom: 2rem !important; }";
const cornerRadius = 28;

async function addRoundedCorners(imagePath: string, radius: number) {
	const dimensions = (await $`magick identify -format %w,%h ${imagePath}`.text()).trim();
	const [widthText, heightText] = dimensions.split(",");
	const width = Number(widthText);
	const height = Number(heightText);

	if (!Number.isFinite(width) || !Number.isFinite(height)) {
		throw new Error(`Unable to detect dimensions for ${imagePath}`);
	}

	const maskPath = `${imagePath}.mask.png`;
	const outputPath = `${imagePath}.rounded.png`;
	const drawCommand = `roundrectangle 0,0,${width - 1},${height - 1},${radius},${radius}`;

	await $`magick -size ${width}x${height} xc:none -fill white -draw ${drawCommand} ${maskPath}`;
	await $`magick ${imagePath} ${maskPath} -alpha off -compose CopyOpacity -composite ${outputPath}`;
	await $`mv ${outputPath} ${imagePath}`;
	await $`rm ${maskPath}`;
}

const screenshots: Array<{ imagePath: string; options: Parameters<typeof captureWebsite.file>[2] }> = [
	{
		imagePath: join(__dirname, "../../data/images/liwan-desktop.png"),
		options: {
			overwrite: true,
			width: 1100,
			height: 1465,
			quality: 0.8,
			styles: [geoCardMargin],
		},
	},
	{
		imagePath: join(__dirname, "../../data/images/liwan-desktop-dark.png"),
		options: {
			darkMode: true,
			overwrite: true,
			width: 1100,
			height: 1465,
			quality: 0.8,
			styles: [geoCardMargin],
		},
	},
	{
		imagePath: join(__dirname, "../../data/images/liwan-desktop-full.png"),
		options: {
			overwrite: true,
			width: 1100,
			fullPage: true,
			quality: 0.8,
		},
	},
	{
		imagePath: join(__dirname, "../../data/images/liwan-desktop-full-dark.png"),
		options: {
			darkMode: true,
			overwrite: true,
			width: 1100,
			fullPage: true,
			quality: 0.8,
		},
	},
];

for (const { imagePath, options } of screenshots) {
	await captureWebsite.file("http://localhost:9042/p/public-project", imagePath, options);
	await addRoundedCorners(imagePath, cornerRadius);
}
