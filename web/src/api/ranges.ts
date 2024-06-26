import {
	addDays,
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
import type { DateRange } from ".";
import type { GraphRange } from "../components/graph";

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
	const end = addDays(endOfDay(new Date()), 1);
	const start = subDays(end, days);
	return { start, end };
};

export const resolveRange = (name: RangeName) => ranges[name]();

// all rangeNames are keys of the ranges object
export const ranges: Record<RangeName, () => { range: DateRange; dataPoints: number; graphRange: GraphRange }> = {
	today: () => {
		const now = new Date();
		const end = endOfHour(now);
		const start = startOfDay(now);
		const hours = differenceInHours(end, start);
		return { range: { start, end }, dataPoints: hours, graphRange: "hour" };
	},
	yesterday: () => {
		const end = startOfDay(new Date());
		const start = subDays(end, 1);
		return { range: { start: start, end: addHours(end, 1) }, dataPoints: 13, graphRange: "hour" };
	},
	last7Days: () => ({ range: lastXDays(7), dataPoints: 7, graphRange: "day" }),
	last30Days: () => ({ range: lastXDays(30), dataPoints: 30, graphRange: "day" }),
	last12Months: () => {
		const now = new Date();
		const start = subMonths(now, 12);
		return { range: { start, end: now }, dataPoints: 12, graphRange: "month" };
	},
	monthToDate: () => {
		const now = new Date();
		const start = startOfMonth(now);
		const days = differenceInDays(now, start);
		return { range: { start, end: now }, dataPoints: days, graphRange: "day" };
	},
	yearToDate: () => {
		const now = new Date();
		const start = startOfYear(now);
		const months = differenceInMonths(now, start);
		return { range: { start, end: now }, dataPoints: months, graphRange: "month" };
	},
};
