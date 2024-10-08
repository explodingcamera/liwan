import {
	addHours,
	differenceInDays,
	differenceInHours,
	differenceInMonths,
	endOfDay,
	endOfHour,
	startOfDay,
	startOfMonth,
	startOfYear,
	subDays,
	subMonths,
} from "date-fns";

import type { DateRange } from "./types";
import type { GraphRange } from "../components/graph/graph";

export const rangeNames = {
	today: "Today",
	yesterday: "Yesterday",
	last7Days: "Last 7 Days",
	last30Days: "Last 30 Days",
	last12Months: "Last 12 Months",
	monthToDate: "Month to Date",
	yearToDate: "Year to Date",
};
export type RangeName = keyof typeof rangeNames;

const lastXDays = (days: number) => {
	const end = endOfDay(new Date()).getTime();
	const start = subDays(end, days).getTime();
	return { start, end };
};

export const resolveRange = (name: RangeName) => ranges[name]();

// all rangeNames are keys of the ranges object
export const ranges: Record<RangeName, () => { range: DateRange; dataPoints: number; graphRange: GraphRange }> = {
	today: () => {
		const now = new Date();
		const end = endOfHour(now).getTime();
		const start = startOfDay(now).getTime();
		const hours = differenceInHours(end, start);
		return { range: { start, end }, dataPoints: hours + 1, graphRange: "hour" };
	},
	yesterday: () => {
		const now = new Date();
		const start = addHours(startOfDay(subDays(now, 1)), 1).getTime();
		const end = addHours(endOfDay(subDays(now, 1)), 1).getTime();
		return { range: { start: start, end }, dataPoints: 24, graphRange: "hour" };
	},
	last7Days: () => ({ range: lastXDays(7), dataPoints: 7, graphRange: "day" }),
	last30Days: () => ({ range: lastXDays(30), dataPoints: 30, graphRange: "day" }),
	last12Months: () => {
		const now = new Date().getTime();
		const start = subMonths(now, 12).getTime();
		return { range: { start, end: now }, dataPoints: 12, graphRange: "month" };
	},
	monthToDate: () => {
		const now = new Date().getTime();
		const start = startOfMonth(now).getTime();
		const days = differenceInDays(now, start);
		return { range: { start, end: now }, dataPoints: days, graphRange: "day" };
	},
	yearToDate: () => {
		const now = new Date().getTime();
		const start = endOfDay(subDays(startOfYear(now), 1)).getTime() - 1000;
		const months = differenceInMonths(now, start);
		return { range: { start, end: now }, dataPoints: months + 1, graphRange: "month" };
	},
};
