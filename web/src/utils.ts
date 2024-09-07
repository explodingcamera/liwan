import type { Metric } from "./api";

type ClassName = string | undefined | null | false;

export const cls = (class1: ClassName | ClassName[], ...classes: (ClassName | ClassName[])[]) =>
	[class1, ...classes.flat()]
		.flat()
		.filter((cls): cls is string => typeof cls === "string" && cls.length > 0)
		.join(" ");

// get the username cookie or undefined if not set
export const getUsername = () => document.cookie.match(/username=(.*?)(;|$)/)?.[1];

export const formatMetricVal = (metric: Metric, value: number) => {
	let res = value;
	if (metric === "avg_views_per_session") {
		res = value / 1000;
	}

	if (res >= 1000) {
		return `${(res / 1000).toFixed(1).replace(/\.0$/, "")}K`;
	}

	if (res >= 1000000) {
		return `${(res / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	}

	return res.toFixed(1).replace(/\.0$/, "") || "0";
};

export const formatPercent = (value: number) => {
	if (value === -1) return "∞";
	if (value >= 10000 || value <= -10000) return `${(value / 100).toFixed(0)}x`;
	if (value >= 1000 || value <= -1000) return `${value.toFixed(0).replace(/\.0$/, "") || "0"}%`;
	return `${value.toFixed(1).replace(/\.0$/, "") || "0"}%`;
};
