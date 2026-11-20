import styles from "./dimensions.module.css";

import type { Metric } from "@/constants";
import { cls } from "@/utils";
import { formatMetricChange, formatMetricVal, isMetricImprovement } from "../metric-format";

export const DimensionMetricValue = ({
	value,
	comparisonValue,
	metric,
	className,
}: {
	value?: number | null;
	comparisonValue?: number | null;
	metric: Metric;
	className?: string;
}) => {
	const change = value != null && comparisonValue != null ? value - comparisonValue : undefined;
	const formattedChange =
		change !== undefined && comparisonValue != null ? formatMetricChange(change, metric, comparisonValue) : null;
	const improved = change === undefined ? undefined : isMetricImprovement(change, metric);

	return (
		<div className={cls(styles.metricValues, className)}>
			<span title={value == null ? "No data or not enough samples" : undefined}>{formatMetricVal(value, metric)}</span>
			{formattedChange !== null && (
				<span className={styles.metricChange} data-improved={improved}>
					{formattedChange}
				</span>
			)}
		</div>
	);
};
