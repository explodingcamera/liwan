import type { Metric } from "@/constants";

/** Format a metric value, optionally using a common unit across a chart axis. */
export const formatMetricVal = (value: number | null | undefined, metric: Metric, scale?: number) => {
	if (value == null) return "—";
	if (metric === "bounce_rate") return `${(Math.floor(value * 1000) / 10).toFixed(1)}%`;
	if (metric === "avg_time_on_site") return formatDuration(value);
	if (value === 0) return "0";

	const unit = scale ?? value;
	if (unit > 999999) return `${(value / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	if (unit > 999) return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`;
	return value.toFixed(1).replace(/\.0$/, "");
};

/** Format a signed metric difference, using relative percent change for bounce rate. */
export const formatMetricChange = (change: number, metric: Metric, previous: number) => {
	if (metric === "bounce_rate" && previous === 0) return null;
	const magnitude = Math.abs(change);
	const sign = change > 0 ? "+" : change < 0 ? "−" : "";
	if (metric === "bounce_rate") return `${sign}${Number(((magnitude / previous) * 100).toFixed(1))}%`;
	if (metric === "avg_time_on_site" && magnitude > 0 && magnitude < 1) return `${sign}<1s`;
	return `${sign}${formatMetricVal(magnitude, metric)}`;
};

/** Format a percent change, abbreviating large changes as a multiplier. */
export const formatPercent = (value: number) => {
	if (Math.abs(value) >= 10000) return `${(value / 100).toFixed(0)}x`;
	if (Math.abs(value) >= 1000) return `${value.toFixed(0)}%`;
	return `${value.toFixed(1).replace(/\.0$/, "")}%`;
};

/** Return whether the change is an improvement, or undefined when unchanged. */
export const isMetricImprovement = (change: number, metric: Metric) =>
	change === 0 ? undefined : metric === "bounce_rate" ? change < 0 : change > 0;

const formatDuration = (value: number) => {
	const totalSeconds = Math.floor(value);
	const hours = Math.floor(totalSeconds / 3600);
	const minutes = Math.floor((totalSeconds % 3600) / 60);
	const remainingSeconds = totalSeconds % 60;
	const parts = hours > 0 ? [hours, minutes, remainingSeconds] : [minutes, remainingSeconds];
	return parts.map((part) => String(part).padStart(2, "0")).join(":");
};
