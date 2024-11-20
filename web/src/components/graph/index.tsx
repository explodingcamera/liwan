import { differenceInSeconds, endOfDay, endOfHour, endOfMonth, endOfYear } from "date-fns";
import _graph from "./graph.module.css";

import { lazy } from "react";
import type { Metric } from "../../api";
import type { DateRange } from "../../api/ranges.ts";

export const LineGraph = lazy(() => import("./graph.tsx").then(({ LineGraph }) => ({ default: LineGraph })));

export type DataPoint = {
	x: Date;
	y: number;
};

export const toDataPoints = (data: number[], range: DateRange, metric: Metric): DataPoint[] => {
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
