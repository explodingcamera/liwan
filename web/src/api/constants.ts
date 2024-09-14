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
	equal: "equals",
	is_null: "is null",
	not_contains: "does not contain",
	not_equal: "does not equal",
};

export const filterNamesCapitalized: Record<DimensionFilter["filterType"], string> = {
	contains: "Contains",
	equal: "Equals",
	is_null: "Is Null",
	not_contains: "Does Not Contain",
	not_equal: "Does Not Equal",
};
