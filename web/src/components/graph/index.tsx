import { differenceInSeconds } from "date-fns";
import _graph from "./graph.module.css";

import { lazy } from "react";
import type { Metric } from "../../api";

export const LineGraph = lazy(() => import("./graph.tsx").then(({ LineGraph }) => ({ default: LineGraph })));

export type DataPoint = {
	x: Date;
	y: number;
};

export const toDataPoints = (data: number[], range: { start: number; end: number }, metric: Metric): DataPoint[] => {
	const step = differenceInSeconds(range.end, range.start) / data.length;
	return data.map((value, i) => ({
		x: new Date(range.start + i * step * 1000 + 1000),
		y: value,
	}));
};
