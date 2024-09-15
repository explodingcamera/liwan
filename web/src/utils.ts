type ClassName = string | undefined | null | false;

export const capitalizeAll = (str: string) => str.replace(/(?:^|\s)\S/g, (a) => a.toUpperCase());

export const cls = (class1: ClassName | ClassName[], ...classes: (ClassName | ClassName[])[]) =>
	[class1, ...classes.flat()]
		.flat()
		.filter((cls): cls is string => typeof cls === "string" && cls.length > 0)
		.join(" ");

// get the username cookie or undefined if not set
export const getUsername = () => document.cookie.match(/username=(.*?)(;|$)/)?.[1];

export const formatMetricVal = (value: number) => {
	if (value >= 1000) {
		return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`;
	}

	if (value >= 1000000) {
		return `${(value / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	}

	return value.toFixed(1).replace(/\.0$/, "") || "0";
};

export const formatPercent = (value: number) => {
	if (value === -1) return "∞";
	if (value >= 10000 || value <= -10000) return `${(value / 100).toFixed(0)}x`;
	if (value >= 1000 || value <= -1000) return `${value.toFixed(0).replace(/\.0$/, "") || "0"}%`;
	return `${value.toFixed(1).replace(/\.0$/, "") || "0"}%`;
};

export const tryParseUrl = (url: string) => {
	try {
		return new URL(url);
	} catch {
		try {
			return new URL(`https://${url}`);
		} catch {
			return url;
		}
	}
};

export const formatHost = (url: string | URL) => {
	if (typeof url === "string") return url;
	return url.hostname;
};

export const formatFullUrl = (url: string | URL) => {
	if (typeof url === "string") return url;
	return `${url.hostname}${url.pathname}${url.search}`;
};

export const getHref = (url: string | URL) => {
	if (typeof url === "string") {
		if (!url.startsWith("http")) return `https://${url}`;
		return url;
	}

	return url.href;
};

export const countryCodeToFlag = (countryCode: string) => {
	const code = countryCode.length === 2 ? countryCode : "XX";
	const codePoints = code
		.toUpperCase()
		.split("")
		.map((char) => 127397 + char.charCodeAt(0));
	return String.fromCodePoint(...codePoints);
};
