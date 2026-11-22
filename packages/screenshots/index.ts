import { join } from "node:path";
import { $ } from "bun";
import puppeteer from "puppeteer-core";

const screenshotStyles = "html { scrollbar-gutter: auto; scrollbar-width: none; }";
const compactStyles = `
	header.page-shell { padding-block: var(--page-gutter) 0.5rem; }
	header.page-shell nav { min-height: 0; }
	.geocard { margin-bottom: var(--page-gutter); }
`;
const fullPageStyles = `
	main.page-shell { padding-bottom: var(--page-gutter); }
	body > footer.footer { display: none; }
`;
const cornerRadius = 28;
const viewport = { width: 1100, height: 1360 };

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
	await page.setViewport(viewport);
	await page.emulateMediaFeatures([
		{ name: "prefers-color-scheme", value: darkMode ? "dark" : "light" },
		{ name: "prefers-reduced-motion", value: "reduce" },
	]);
	// Apply capture styles before the dashboard measures its charts.
	await page.evaluateOnNewDocument(
		(styles) => {
			const sheet = new CSSStyleSheet();
			sheet.replaceSync(styles);
			document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
		},
		`${screenshotStyles} ${fullPage ? fullPageStyles : compactStyles}`,
	);
	await page.goto("https://demo.liwan.dev/p/liwan.dev", { waitUntil: "networkidle2" });
	await page.waitForSelector(".geocard", { visible: true });
	await page.evaluate(async () => {
		await document.fonts.ready;
	});

	const imagePath = join(__dirname, "../../data/images", name);
	if (fullPage) {
		await page.screenshot({ path: imagePath, fullPage: true });
	} else {
		const height = await page.$eval(".geocard", (card) =>
			Math.ceil(card.getBoundingClientRect().bottom + Number.parseFloat(getComputedStyle(card).marginBottom)),
		);
		await page.screenshot({ path: imagePath, clip: { x: 0, y: 0, width: viewport.width, height } });
	}
	await page.close();
	await addRoundedCorners(imagePath, cornerRadius);
}
await browser.close();
