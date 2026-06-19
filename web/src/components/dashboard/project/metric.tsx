import styles from "./metric.module.css";

import { TrendingDownIcon, TrendingUpIcon } from "lucide-react";

import type { Metric, StatsResponse } from "@/constants";
import { metrics as defaultMetrics } from "@/constants";
import { cls, formatMetricVal, formatPercent } from "@/utils";
import { CardButton } from "./card";

export const SelectMetrics = ({
	data,
	metric,
	metrics = defaultMetrics,
	setMetric,
	className,
}: {
	data?: StatsResponse;
	metric: Metric;
	metrics?: readonly Metric[];
	setMetric: (value: Metric) => void;
	className?: string;
}) => {
	const visible = (metric: Metric) => metrics.includes(metric);

	return (
		<div className={cls(styles.metrics, className)}>
			{visible("views") && (
				<SelectMetric
					title="Total Views"
					value={data?.stats.totalViews}
					prevValue={data?.statsPrev.totalViews}
					metric={"views"}
					onSelect={() => setMetric("views")}
					selected={metric === "views"}
				/>
			)}
			{visible("unique_visitors") && (
				<SelectMetric
					title="Unique Visitors"
					value={data?.stats.uniqueVisitors}
					prevValue={data?.statsPrev.uniqueVisitors}
					metric={"unique_visitors"}
					onSelect={() => setMetric("unique_visitors")}
					selected={metric === "unique_visitors"}
				/>
			)}
			{visible("avg_time_on_site") && (
				<SelectMetric
					title="Avg. Time on Site"
					value={data?.stats.avgTimeOnSite}
					prevValue={data?.statsPrev.avgTimeOnSite}
					metric={"avg_time_on_site"}
					onSelect={() => setMetric("avg_time_on_site")}
					selected={metric === "avg_time_on_site"}
				/>
			)}
			{visible("bounce_rate") && (
				<SelectMetric
					title="Bounce Rate"
					value={data?.stats.bounceRate}
					prevValue={data?.statsPrev.bounceRate}
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
	prevValue = 0,
	metric,
	onSelect,
	selected,
}: {
	title: string;
	value?: number | null;
	metric: Metric;
	prevValue?: number | null;
	decimals?: number;
	onSelect: () => void;
	selected: boolean;
}) => {
	const unavailable = value == null || prevValue == null;
	const currentValue = value ?? 0;
	const previousValue = prevValue ?? 0;
	const change = currentValue - previousValue;
	const changePercent =
		metric === "bounce_rate" ? change * 100 : prevValue ? (change / prevValue) * 100 : value ? -1 : 0;
	const changeIsGood = metric === "bounce_rate" ? change < 0 : change > 0;
	const color = change === 0 ? "gray" : changeIsGood ? "#22c55e" : "red";
	const icon = change > 0 ? <TrendingUpIcon size={14} /> : change < 0 ? <TrendingDownIcon size={14} /> : "—";
	const formattedChange = changePercent > 0 ? `+${formatPercent(changePercent)}` : formatPercent(changePercent);

	return (
		<CardButton onClick={onSelect} active={selected} className={styles.metric}>
			<h2>{title}</h2>
			<h3>
				{unavailable ? "Unavailable" : formatMetricVal(currentValue, metric)}
				<span style={{ color }} className={styles.change}>
					{!unavailable && (
						<>
							{icon} {formattedChange}
						</>
					)}
				</span>
			</h3>
		</CardButton>
	);
};
