import styles from "./linegraph.module.css";

import { lazy, useEffect, useState } from "react";

import type { DateRange } from "@/api/ranges.ts";
import type { GraphResponse, Metric } from "@/constants.ts";

export type { GraphRange } from "./linegraph.tsx";

const LineGraphInner = lazy(() => import("./linegraph.tsx").then(({ LineGraph }) => ({ default: LineGraph })));

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

	useEffect(() => {
		if (data) {
			setLineGraphState({
				data,
				title,
				metric,
			});
		}
	}, [data, title, metric]);

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
