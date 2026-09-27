type RuntimeConfig = {
	baseUrl: string;
	disableFavicons: boolean;
};

const readConfig = (): RuntimeConfig | undefined => {
	if (typeof document === "undefined") return undefined;

	const text = document.getElementById("liwan-config")?.textContent;
	if (!text) return undefined;

	try {
		const config = JSON.parse(text) as Partial<RuntimeConfig>;
		if (typeof config.baseUrl === "string" && typeof config.disableFavicons === "boolean") {
			return { baseUrl: config.baseUrl, disableFavicons: config.disableFavicons };
		}
	} catch {
		return undefined;
	}

	return undefined;
};
export const runtimeConfig = readConfig();

export const basePath = runtimeConfig ? new URL(runtimeConfig.baseUrl).pathname.replace(/\/$/, "") : "";

export const appPath = (path: string) => `${basePath}${path}`;
