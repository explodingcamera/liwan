import type { DateRange } from ".";

export const rangeNames = {
	today: "Today",
	yesterday: "Yesterday",
	last7Days: "Last 7 Days",
	last30Days: "Last 30 Days",
	last12Months: "Last 12 Months",
	monthToDate: "Month to Date",
	yearToDate: "Year to Date",
};

const lastXDays = (days: number) => {
	const end = new Date();
	const start = new Date(end);
	start.setDate(start.getDate() - days);
	return { start, end };
};

// all rangeNames are keys of the ranges object
export const ranges: Record<keyof typeof rangeNames, () => DateRange> = {
	today: () => {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		return { start, end: now };
	},
	yesterday: () => {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
		const end = new Date(start);
		end.setDate(end.getDate() + 1);
		return { start, end };
	},
	last7Days: () => lastXDays(7),
	last30Days: () => lastXDays(30),
	last12Months: () => {
		const now = new Date();
		const start = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate());
		return { start, end: now };
	},
	monthToDate: () => {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), 1);
		return { start, end: now };
	},
	yearToDate: () => {
		const now = new Date();
		const start = new Date(now.getFullYear(), 0, 1);
		return { start, end: now };
	},
};
