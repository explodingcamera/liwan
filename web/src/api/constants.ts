import type { Dimension, DimensionFilter, Metric } from "./types";

export const metricNames: Record<Metric, string> = {
	views: "Total Views",
	sessions: "Total Sessions",
	unique_visitors: "Unique Visitors",
	avg_views_per_session: "Avg. Views Per Session",
};

export const dimensionNames: Record<Dimension, string> = {
	platform: "Platform",
	browser: "Browser",
	url: "URL",
	path: "Path",
	mobile: "Device Type",
	referrer: "Referrer",
	city: "City",
	country: "Country",
	fqdn: "Domain",
};

export const filterNames: Record<DimensionFilter["filterType"], string> = {
	contains: "contains",
	equal: "is",
	is_null: "is null",
	ends_with: "ends with",
	is_false: "is false",
	is_true: "is true",
	starts_with: "starts with",
};

export const filterNamesInverted: Record<DimensionFilter["filterType"], string> = {
	contains: "does not contain",
	equal: "is not",
	is_null: "is not null",
	ends_with: "does not end with",
	is_false: "is not false",
	is_true: "is not true",
	starts_with: "does not start with",
};

export const capitalizeAll = (str: string) => str.replace(/(?:^|\s)\S/g, (a) => a.toUpperCase());
