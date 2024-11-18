import {
	addHours,
	differenceInDays,
	differenceInHours,
	differenceInMonths,
	endOfDay,
	endOfHour,
	endOfYear,
	startOfDay,
	startOfMonth,
	startOfYear,
	subDays,
	subMonths,
	subYears,
} from "date-fns";

import type { DateRange } from "./types";
import type { GraphRange } from "../components/graph/graph";

export const rangeEndsToday = (range: DateRange) => {
	const now = new Date().getTime();
	return startOfDay(now) === startOfDay(range.end);
};

export const rangeGraphRange = (range: DateRange): GraphRange => {
	const days = differenceInDays(range.end, range.start);
	if (days < 2) {
		return "hour";
	}
	const months = differenceInMonths(range.end, range.start);
	if (months < 2) {
		return "day";
	}
	return "month";
};

export const rangeDataPoints = (range: DateRange): number => {
	switch (rangeGraphRange(range)) {
		case "hour":
			return differenceInHours(range.end, range.start) + 1;
		case "day":
			return differenceInDays(range.end, range.start) + 1;
		case "month":
			return differenceInMonths(range.end, range.start) + 1;
	}
	throw new Error("unreachable");
};

export const serializeRange = (range: DateRange): string => {
	const start = new Date(range.start);
	const end = new Date(range.end);
	return `${Number(start)}:${Number(end)}`;
};

export const deserializeRange = (range: string): DateRange => {
	if (!range.includes(":")) {
		return ranges[range as RangeName]().range;
	}

	const [start, end] = range.split(":").map(Number);
	return { start, end };
};

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

export const previusRange = (range: string) => {
	if (range === "today") return "yesterday";
	if (range === "yearToDate") {
		const lastYear = subYears(new Date(), 1);
		const start = startOfYear(lastYear).getTime();
		const end = endOfYear(lastYear).getTime();
		return serializeRange({ start, end });
	}
	const r = deserializeRange(range);
	const size = r.end - r.start;
	const start = r.start - size;
	const end = r.end - size;
	return serializeRange({ start: startOfDay(start).getTime(), end: endOfDay(end).getTime() });
};

export const nextRange = (range: string) => {
	if (range === "yesterday") return "today";
	const r = deserializeRange(range);
	const size = r.end - r.start;
	const start = r.start + size;
	const end = r.end + size;
	return serializeRange({ start: startOfDay(start).getTime(), end: endOfDay(end).getTime() });
};
