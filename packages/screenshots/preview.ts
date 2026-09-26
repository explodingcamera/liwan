import { join } from "node:path";
import puppeteer from "puppeteer-core";

const previewUrl = new URL("./preview.html", import.meta.url).href;
const outputPath = join(__dirname, "../../data/images/liwan-social-preview.png");

const executablePath = Bun.which("google-chrome") ?? Bun.which("google-chrome-stable");
if (!executablePath) throw new Error("google-chrome or google-chrome-stable not found on PATH");
const browser = await puppeteer.launch({ executablePath });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 640 });
await page.goto(previewUrl, { waitUntil: "networkidle2" });
await new Promise((resolve) => setTimeout(resolve, 1000));
await page.screenshot({ path: outputPath });
await browser.close();
