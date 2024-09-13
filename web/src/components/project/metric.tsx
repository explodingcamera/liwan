import styles from "./metric.module.css";
import { TrendingDownIcon, TrendingUpIcon } from "lucide-react";

import type { Metric, StatsResponse } from "../../api";
import { cls, formatMetricVal, formatPercent } from "../../utils";
import { CardButton } from "../card";

export const SelectMetrics = ({
	data,
	metric,
	setMetric,
	className,
}: {
	data?: StatsResponse;
	metric: Metric;
	setMetric: (value: Metric) => void;
	className?: string;
}) => {
	return (
		<div className={cls(styles.metrics, className)}>
			<SelectMetric
				title="Total Views"
				value={data?.stats.totalViews}
				prevValue={data?.statsPrev.totalViews}
				metric={"views"}
				onSelect={() => setMetric("views")}
				selected={metric === "views"}
			/>
			<SelectMetric
				title="Unique Visitors"
				value={data?.stats.uniqueVisitors}
				prevValue={data?.statsPrev.uniqueVisitors}
				metric={"unique_visitors"}
				onSelect={() => setMetric("unique_visitors")}
				selected={metric === "unique_visitors"}
			/>
			<SelectMetric
				title="Total Sessions"
				value={data?.stats.totalSessions}
				prevValue={data?.statsPrev.totalSessions}
				metric={"sessions"}
				onSelect={() => setMetric("sessions")}
				selected={metric === "sessions"}
			/>
			<SelectMetric
				title="Avg. Views Per Session"
				value={data?.stats.avgViewsPerSession}
				prevValue={data?.statsPrev.avgViewsPerSession}
				metric={"avg_views_per_session"}
				onSelect={() => setMetric("avg_views_per_session")}
				selected={metric === "avg_views_per_session"}
			/>
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
	value?: number;
	metric: Metric;
	prevValue?: number;
	decimals?: number;
	onSelect: () => void;
	selected: boolean;
}) => {
	const change = value - prevValue;
	const changePercent = prevValue ? (change / prevValue) * 100 : value ? -1 : 0;
	const color = change > 0 ? "#22c55e" : change < 0 ? "red" : "gray";
	const icon = change > 0 ? <TrendingUpIcon size={14} /> : change < 0 ? <TrendingDownIcon size={14} /> : "—";

	return (
		<CardButton onClick={onSelect} active={selected} className={styles.metric}>
			<h2>{title}</h2>
			<h3>
				{formatMetricVal(value)}
				<span style={{ color }} className={styles.change}>
					{icon} {formatPercent(changePercent)}
				</span>
			</h3>
		</CardButton>
	);
};
