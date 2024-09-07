import type { Dimension, Metric } from "./types";

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
