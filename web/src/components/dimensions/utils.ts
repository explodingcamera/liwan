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
