import { differenceInSeconds, endOfDay, endOfHour, endOfMonth, endOfYear } from "date-fns";

import { lazy, useEffect, useState } from "react";
import type { DateRange } from "../../api/ranges.ts";
import type { Metric } from "../../api/types.ts";

import styles from "./graph.module.css";

const LineGraphInner = lazy(() => import("./graph.tsx").then(({ LineGraph }) => ({ default: LineGraph })));

export const LineGraph = ({
	isLoading,
	isUpdating,
	data,
	title,
	metric,
	range,
}: {
	data?: DataPoint[];
	isLoading?: boolean;
	isUpdating?: boolean;
	title: string;
	metric: Metric;
	range: DateRange;
}) => {
	const [lineGraphState, setLineGraphState] = useState<GraphState | undefined>(undefined);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we only want to update the graph when the graph data changes
	useEffect(() => {
		if (data) {
			setLineGraphState({
				data,
				title,
				metric,
			});
		}
	}, [data]);

	return (
		<div className={styles.graphContainer}>
			{(isLoading || isUpdating) && (
				<div className={styles.updatingOverlay} aria-busy="true" data-no-delay={isLoading}></div>
			)}
			<LineGraphInner
				state={
					lineGraphState || {
						data: [],
						title,
						metric,
					}
				}
				range={range}
			/>
		</div>
	);
};

export type DataPoint = {
	x: Date;
	y: number;
};

export type GraphState = {
	data: DataPoint[];
	title: string;
	metric: Metric;
};

export const toDataPoints = (data: number[], range: DateRange): DataPoint[] => {
	const step = differenceInSeconds(range.value.end, range.value.start) / data.length;
	return data
		.map((value, i) => ({
			x: new Date(range.value.start.getTime() + i * step * 1000 + 1000),
			y: value,
		}))
		.filter((p) => {
			if (range.getGraphRange() === "hour") {
				// filter out points after this hour
				return p.x < endOfHour(new Date());
			}
			if (range.getGraphRange() === "day") {
				// filter out points after today
				return p.x < endOfDay(new Date());
			}
			if (range.getGraphRange() === "month") {
				// filter out points after this month
				return p.x < endOfMonth(new Date());
			}
			if (range.getGraphRange() === "year") {
				// filter out points after this year
				return p.x < endOfYear(new Date());
			}
			return true;
		});
};
