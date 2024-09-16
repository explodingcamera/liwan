import captureWebsite from "capture-website";
import { join } from "node:path";

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop.png"),
	{
		overwrite: true,
		width: 1100,
		height: 1460,
		quality: 0.8,
	},
);

await captureWebsite.file(
	"http://localhost:4321/p/public-project",
	join(__dirname, "../../data/images/liwan-desktop-dark.png"),
	{
		darkMode: true,
		overwrite: true,
		width: 1100,
		height: 1460,
		quality: 0.8,
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
