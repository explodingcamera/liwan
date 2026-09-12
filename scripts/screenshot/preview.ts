import { join } from "node:path";
import captureWebsite from "capture-website";

const previewUrl = new URL("./preview.html", import.meta.url).href;
const outputPath = join(__dirname, "../../data/images/liwan-social-preview.png");

await captureWebsite.file(previewUrl, outputPath, {
	overwrite: true,
	width: 1280,
	height: 640,
	scaleFactor: 1,
	delay: 1,
});
