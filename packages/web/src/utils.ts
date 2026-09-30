import {
	endOfDay,
	endOfMonth,
	endOfQuarter,
	endOfYear,
	format,
	getQuarter,
	isSameDay,
	isSameMinute,
	isSameMonth,
	isSameYear,
	startOfDay,
	startOfMonth,
	startOfQuarter,
	startOfYear,
} from "date-fns";

import type { Metric } from "./constants";

type ClassName = string | undefined | null | false;

export const capitalizeAll = (str: string) => str.replace(/(?:^|\s)\S/g, (a) => a.toUpperCase());

export const cls = (class1: ClassName | ClassName[], ...classes: (ClassName | ClassName[])[]) =>
	[class1, ...classes.flat()]
		.flat()
		.filter((cls): cls is string => typeof cls === "string" && cls.length > 0)
		.join(" ");

// get the username cookie or undefined if not set
export const getUsername = () =>
	typeof document !== "undefined" ? document.cookie.match(/liwan-username=(.*?)(;|$)/)?.[1] : undefined;

export const formatMetricValEvenly = (value: number, metric: Metric, biggest: number) => {
	if (metric === "bounce_rate") return `${(Math.floor(value * 1000) / 10).toFixed(1)}%`;
	if (metric === "avg_time_on_site") return formatDuration(value);
	if (value === 0) return "0";

	if (biggest > 999999) {
		return `${(value / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	}

	if (biggest > 999) {
		return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`;
	}

	return value.toFixed(1).replace(/\.0$/, "") || "0";
};

export const formatMetricVal = (value: number, metric: Metric) => {
	if (metric === "bounce_rate") return `${(Math.floor(value * 1000) / 10).toFixed(1)}%`;
	if (metric === "avg_time_on_site") return formatDuration(value);

	if (value > 999999) {
		return `${(value / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	}

	if (value > 999) {
		return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`;
	}

	return value.toFixed(1).replace(/\.0$/, "") || "0";
};

export const formatPercent = (value: number) => {
	if (value === -1) return "∞";
	if (value >= 10000 || value <= -10000) return `${(value / 100).toFixed(0)}x`;
	if (value >= 1000 || value <= -1000) return `${value.toFixed(0).replace(/\.0$/, "") || "0"}%`;
	return `${value.toFixed(1).replace(/\.0$/, "") || "0"}%`;
};

export const formatDuration = (value: number) => {
	const totalSeconds = Math.floor(value);
	const hours = Math.floor(totalSeconds / 3600);
	const minutes = Math.floor((totalSeconds % 3600) / 60);
	const remainingSeconds = totalSeconds % 60;

	if (hours > 0) {
		return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(remainingSeconds).padStart(2, "0")}`;
	}

	return `${String(minutes).padStart(2, "0")}:${String(remainingSeconds).padStart(2, "0")}`;
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

export const formatPath = (url: string | URL) => {
	if (typeof url === "string") return url;
	return url.pathname;
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

// Case-insensitive fuzzy search. Each space-separated word must appear in order in one of the keys.
// Substring matches rank first (earlier is better), then the tightest in-order match. Ties keep their order.
export const fuzzyFilter = <T>(query: string, items: T[], keys: (keyof T)[]) => {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	const score = (text: string, word: string) => {
		const index = text.indexOf(word);
		if (index !== -1) return index;

		let start = -1;
		let pos = -1;
		for (const char of word) {
			pos = text.indexOf(char, pos + 1);
			if (pos === -1) return Number.POSITIVE_INFINITY;
			if (start === -1) start = pos;
		}
		return text.length + pos - start;
	};

	return items
		.map((item) => {
			const texts = keys.map((key) => String(item[key] ?? "").toLowerCase());
			const total = words.reduce((sum, word) => sum + Math.min(...texts.map((text) => score(text, word))), 0);
			return { item, score: total };
		})
		.filter((result) => result.score !== Number.POSITIVE_INFINITY)
		.sort((a, b) => a.score - b.score)
		.map((result) => result.item);
};

// Date range formatting adapted from little-date (https://github.com/vercel/little-date)
// Copyright (c) 2024 Vercel, MIT License
const formatTime = (date: Date, locale: string) => {
	let text = date.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
	text = text.replace(/ AM/g, "am").replace(/ PM/g, "pm");
	if (text.includes("m")) text = text.replace(/:00/g, "");
	return text.replace(/^0/, "");
};

export const formatDateRange = (
	from: Date,
	to: Date,
	{
		today = new Date(),
		locale = typeof window === "undefined" ? "en-US" : window.navigator.language,
	}: { today?: Date; locale?: string } = {},
) => {
	const sameYear = isSameYear(from, to);
	const sameMonth = isSameMonth(from, to);
	const yearSuffix = isSameYear(from, today) ? "" : `, ${format(to, "yyyy")}`;
	const startTime = isSameMinute(startOfDay(from), from) ? "" : `, ${formatTime(from, locale)}`;
	const endTime = isSameMinute(endOfDay(to), to) ? "" : `, ${formatTime(to, locale)}`;

	// Example: 2023
	if (isSameMinute(startOfYear(from), from) && isSameMinute(endOfYear(to), to)) {
		return format(from, "yyyy");
	}

	// Example: Q1 2023
	if (
		isSameMinute(startOfQuarter(from), from) &&
		isSameMinute(endOfQuarter(to), to) &&
		getQuarter(from) === getQuarter(to)
	) {
		return `Q${getQuarter(from)} ${format(from, "yyyy")}`;
	}

	// Example: January 2023, Jan - Feb 2023
	if (isSameMinute(startOfMonth(from), from) && isSameMinute(endOfMonth(to), to)) {
		if (sameMonth && sameYear) return format(from, "LLLL yyyy");
		return `${format(from, "LLL")} - ${format(to, "LLL yyyy")}`;
	}

	// Example: Jan 1 '23 - Feb 12 '24
	if (!sameYear) {
		return `${format(from, "LLL d ''yy")}${startTime} - ${format(to, "LLL d ''yy")}${endTime}`;
	}

	// Example: Jan 1, 12pm - Feb 2, 1pm[, 2023]
	if (!sameMonth || (!isSameDay(from, to) && (startTime || endTime))) {
		return `${format(from, "LLL d")}${startTime} - ${format(to, "LLL d")}${endTime}${yearSuffix}`;
	}

	// Example: Jan 1 - 12[, 2023]
	if (!isSameDay(from, to)) {
		return `${format(from, "LLL d")} - ${format(to, "d")}${yearSuffix}`;
	}

	// Example: 12pm - 1pm, Jan 1, 12pm - 1pm[, 2023]
	if (startTime || endTime) {
		if (isSameDay(from, today)) return `${formatTime(from, locale)} - ${formatTime(to, locale)}`;
		return `${format(from, "LLL d")}${startTime} - ${formatTime(to, locale)}${yearSuffix}`;
	}

	// Example: Fri, Jan 1[, 2023]
	return `${format(from, "eee, LLL d")}${yearSuffix}`;
};
