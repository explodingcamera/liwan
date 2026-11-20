import cardStyles from "./card.module.css";
import styles from "./metric.module.css";

import { TrendingDownIcon, TrendingUpIcon } from "lucide-react";

import type { Metric, StatsResponse } from "@/constants";
import { metrics as defaultMetrics } from "@/constants";
import { cls } from "@/utils";
import { formatMetricChange, formatMetricVal, formatPercent, isMetricImprovement } from "./metric-format";

export const SelectMetrics = ({
	data,
	metric,
	metrics = defaultMetrics,
	setMetric,
	className,
	isLoading = false,
	eventName = "pageview",
	showAbsoluteChange,
}: {
	data?: StatsResponse;
	metric: Metric;
	metrics?: readonly Metric[];
	setMetric: (value: Metric) => void;
	className?: string;
	isLoading?: boolean;
	eventName?: string;
	showAbsoluteChange?: boolean;
}) => {
	const visible = (metric: Metric) => metrics.includes(metric);

	return (
		<div
			className={cls(styles.metrics, !data && styles.empty, className)}
			aria-busy={isLoading}
			data-loading={isLoading}
		>
			{visible("views") && (
				<SelectMetric
					title={eventName === "pageview" ? "Total Views" : "Completions"}
					value={data?.reports[0]?.totalViews}
					prevValue={data?.reports[1]?.totalViews}
					showAbsoluteChange={showAbsoluteChange}
					metric={"views"}
					onSelect={() => setMetric("views")}
					selected={metric === "views"}
				/>
			)}
			{visible("unique_visitors") && (
				<SelectMetric
					title={eventName === "pageview" ? "Unique Visitors" : "Uniques"}
					value={data?.reports[0]?.uniqueVisitors}
					prevValue={data?.reports[1]?.uniqueVisitors}
					showAbsoluteChange={showAbsoluteChange}
					metric={"unique_visitors"}
					onSelect={() => setMetric("unique_visitors")}
					selected={metric === "unique_visitors"}
				/>
			)}
			{visible("avg_time_on_site") && (
				<SelectMetric
					title="Avg. Time on Site"
					value={data?.reports[0]?.avgTimeOnSite}
					prevValue={data?.reports[1]?.avgTimeOnSite}
					showAbsoluteChange={showAbsoluteChange}
					metric={"avg_time_on_site"}
					onSelect={() => setMetric("avg_time_on_site")}
					selected={metric === "avg_time_on_site"}
				/>
			)}
			{visible("bounce_rate") && (
				<SelectMetric
					title="Bounce Rate"
					value={data?.reports[0]?.bounceRate}
					prevValue={data?.reports[1]?.bounceRate}
					showAbsoluteChange={showAbsoluteChange}
					metric={"bounce_rate"}
					onSelect={() => setMetric("bounce_rate")}
					selected={metric === "bounce_rate"}
				/>
			)}
		</div>
	);
};

export const SelectMetric = ({
	title,
	value = 0,
	prevValue,
	showAbsoluteChange = false,
	metric,
	onSelect,
	selected,
}: {
	title: string;
	value?: number | null;
	metric: Metric;
	prevValue?: number | null;
	showAbsoluteChange?: boolean;
	onSelect: () => void;
	selected: boolean;
}) => {
	const unavailable = value == null;
	const currentValue = value ?? 0;
	const previousValue = prevValue ?? 0;
	const change = currentValue - previousValue;
	const changePercent = previousValue > 0 ? (change / previousValue) * 100 : null;
	const formattedChange =
		showAbsoluteChange && metric !== "bounce_rate"
			? formatMetricChange(change, metric, previousValue)
			: changePercent !== null
				? `${changePercent > 0 ? "+" : ""}${formatPercent(changePercent)}`
				: null;
	const improved = isMetricImprovement(change, metric);
	const TrendIcon = change > 0 ? TrendingUpIcon : change < 0 ? TrendingDownIcon : null;

	return (
		<button
			type="button"
			className={cls(styles.metric, cardStyles.card)}
			onClick={onSelect}
			data-active={selected}
			title={!unavailable && prevValue != null ? `Comparison: ${formatMetricVal(previousValue, metric)}` : undefined}
		>
			<h2>{title}</h2>
			<h3>
				<span className={styles.currentValue}>
					{unavailable ? "Unavailable" : formatMetricVal(currentValue, metric)}
				</span>
				{!unavailable && prevValue != null && formattedChange !== null && (
					<span className={styles.change} data-improved={improved}>
						{TrendIcon && <TrendIcon size={14} aria-hidden="true" />}
						{formattedChange}
					</span>
				)}
			</h3>
		</button>
	);
};
