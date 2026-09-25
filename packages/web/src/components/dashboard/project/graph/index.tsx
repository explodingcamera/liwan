import styles from "./linegraph.module.css";

import { lazy, Suspense } from "react";

import type { DateRange } from "@/api/ranges.ts";
import { LoadingSpinner } from "@/components/ui/loading";
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
	const loading = isLoading || isUpdating;

	return (
		<div className={styles.graphContainer}>
			{loading && (
				<div
					className={styles.updatingOverlay}
					role="status"
					aria-busy="true"
					aria-label={isLoading ? "Loading graph" : "Updating graph"}
					data-no-delay={isLoading}
				>
					<LoadingSpinner immediate />
				</div>
			)}
			<Suspense
				fallback={
					loading ? null : (
						<div
							className={styles.updatingOverlay}
							role="status"
							aria-busy="true"
							aria-label="Loading graph"
							data-no-delay="true"
						>
							<LoadingSpinner immediate />
						</div>
					)
				}
			>
				<LineGraphInner data={data ?? []} title={title} metric={metric} range={range} />
			</Suspense>
		</div>
	);
};

export type DataPoint = {
	x: Date;
	y: number;
};

export const toDataPoints = (data: GraphResponse["data"]): DataPoint[] => {
	return data.map((point) => ({
		x: new Date(point.binStart),
		y: point.value,
	}));
};
