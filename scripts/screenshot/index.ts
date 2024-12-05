import captureWebsite from "capture-website";
import { join } from "node:path";

const geoCardMargin = ".geocard { margin-bottom: 2rem !important; }";

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop.png"),
	{
		overwrite: true,
		width: 1100,
		height: 1480,
		quality: 0.8,
		styles: [geoCardMargin],
	},
);

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop-dark.png"),
	{
		darkMode: true,
		overwrite: true,
		width: 1100,
		height: 1480,
		quality: 0.8,
		styles: [geoCardMargin],
	},
);

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop-full.png"),
	{
		overwrite: true,
		width: 1100,
		fullPage: true,
		quality: 0.8,
	},
);

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop-full-dark.png"),
	{
		darkMode: true,
		overwrite: true,
		width: 1100,
		fullPage: true,
		quality: 0.8,
	},
);
