import { join } from "node:path";
import { $ } from "bun";
import puppeteer from "puppeteer-core";

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

const executablePath = Bun.which("google-chrome") ?? Bun.which("google-chrome-stable");
if (!executablePath) throw new Error("google-chrome or google-chrome-stable not found on PATH");
const browser = await puppeteer.launch({ executablePath });
for (const [name, darkMode, fullPage] of [
	["liwan-desktop.png", false, false],
	["liwan-desktop-dark.png", true, false],
	["liwan-desktop-full.png", false, true],
	["liwan-desktop-full-dark.png", true, true],
] as const) {
	const page = await browser.newPage();
	await page.setViewport({ width: 1100, height: 1445 });
	await page.emulateMediaFeatures([{ name: "prefers-color-scheme", value: darkMode ? "dark" : "light" }]);
	await page.goto("https://demo.liwan.dev/p/liwan.dev", { waitUntil: "networkidle2" });
	if (!fullPage) await page.addStyleTag({ content: geoCardMargin });
	const imagePath = join(__dirname, "../../data/images", name);
	await page.screenshot({ path: imagePath, fullPage });
	await page.close();
	await addRoundedCorners(imagePath, cornerRadius);
}
await browser.close();
