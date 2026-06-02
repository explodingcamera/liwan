import styles from "./graph.module.css";

import { lazy, useEffect, useState } from "react";

import type { DateRange } from "../../api/ranges.ts";
import type { GraphResponse, Metric } from "../../constants.ts";

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

export const toDataPoints = (data: GraphResponse["data"]): DataPoint[] => {
	return data.map((point) => ({
		x: new Date(point.binStart),
		y: point.value,
	}));
};
