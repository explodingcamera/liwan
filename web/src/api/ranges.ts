import {
	addDays,
	addHours,
	addMonths,
	addSeconds,
	addWeeks,
	addYears,
	differenceInDays,
	differenceInHours,
	differenceInSeconds,
	differenceInYears,
	endOfDay,
	endOfMonth,
	endOfWeek,
	endOfYear,
	isAfter,
	isEqual,
	isSameDay,
	isSameMonth,
	isSameWeek,
	isSameYear,
	startOfDay,
	startOfMonth,
	startOfWeek,
	startOfYear,
	subDays,
	subMonths,
	subSeconds,
	subWeeks,
	subYears,
} from "date-fns";

import type { GraphRange } from "../components/graph/graph";
import { formatDateRange } from "little-date";

type DateRangeValue = { start: Date; end: Date };

export class DateRange {
	#value: RangeName | { start: Date; end: Date };
	label: string;

	constructor(value: RangeName | { start: Date; end: Date }) {
		this.#value = value;
		this.label = "";
	}

	get value(): DateRangeValue {
		if (typeof this.#value === "string") {
			return ranges[this.#value as RangeName]().range;
		}
		return this.#value as DateRangeValue;
	}

	isCustom(): boolean {
		return typeof this.#value !== "string";
	}

	format(): string {
		if (typeof this.#value === "string") return wellKnownRanges[this.#value];
		return formatDateRange(this.#value.start, this.#value.end);
	}

	cacheKey(): string {
		return this.serialize();
	}

	serialize(): string {
		if (typeof this.#value === "string") return this.#value;
		return `${Number(this.#value.start)}:${Number(this.#value.end)}`;
	}

	static deserialize(range: string): DateRange {
		if (!range.includes(":")) {
			return new DateRange(range as RangeName);
		}
		const [start, end] = range.split(":").map((v) => new Date(Number(v)));
		return new DateRange({ start, end });
	}

	endsToday(): boolean {
		// ends today or ends in the future
		return isEqual(endOfDay(new Date()), endOfDay(this.value.end)) || this.value.end > new Date();
	}

	toAPI(): { start: string; end: string } {
		const start = this.value.start.toISOString();
		const end = this.value.end.toISOString();
		return { start, end };
	}

	getGraphRange(): GraphRange {
		if (differenceInDays(this.value.end, this.value.start) < 7) return "hour";
		return "day";
	}

	getAxisRange(): "hour" | "day" | "day+year" {
		if (differenceInDays(this.value.end, this.value.start) < 1) return "hour";
		if (differenceInYears(this.value.end, addHours(this.value.start, 1)) > 1) return "day+year";
		return "day";
	}

	getTooltipRange(): "hour" | "day+hour" | "day" {
		if (differenceInDays(this.value.end, this.value.start) < 1) return "hour";
		if (differenceInDays(this.value.end, this.value.start) < 6) return "day+hour";
		return "day";
	}

	getGraphDataPoints(): number {
		const diff = differenceInDays(this.value.end, this.value.start);

		if (diff >= 6) return diff;
		if (diff >= 1) return differenceInHours(this.value.end, this.value.start);
		return 24;
	}

	#isDayBeforeYesterday() {
		return isSameDay(subDays(new Date(), 2), this.value.start) && isSameDay(subDays(new Date(), 2), this.value.end);
	}

	previous() {
		if (this.#value === "today") return new DateRange("yesterday");

		if (
			isEqual(startOfWeek(this.value.start), this.value.start) &&
			isEqual(endOfWeek(this.value.end), this.value.end) &&
			isSameWeek(this.value.start, this.value.end)
		) {
			const start = subWeeks(this.value.start, 1);
			const end = subWeeks(this.value.end, 1);
			return new DateRange({ start, end });
		}

		if (
			isEqual(startOfMonth(this.value.start), this.value.start) &&
			isEqual(endOfMonth(this.value.end), this.value.end) &&
			isSameMonth(this.value.start, this.value.end)
		) {
			const start = startOfMonth(subMonths(this.value.start, 1));
			const end = endOfMonth(subMonths(this.value.end, 1));
			return new DateRange({ start, end });
		}

		if (
			isEqual(startOfYear(this.value.start), this.value.start) &&
			isEqual(endOfYear(this.value.end), this.value.end) &&
			isSameYear(this.value.start, this.value.end)
		) {
			const start = startOfYear(subYears(this.value.start, 1));
			const end = endOfYear(subYears(this.value.end, 1));
			return new DateRange({ start, end });
		}

		if (differenceInHours(this.value.end, this.value.start) < 23) {
			const start = subSeconds(this.value.start, differenceInSeconds(this.value.end, this.value.start));
			const end = subSeconds(this.value.end, differenceInSeconds(this.value.end, this.value.start));
			return new DateRange({ start, end });
		}

		const size = differenceInDays(this.value.end, this.value.start);
		const start = subDays(this.value.start, size + 1);
		const end = subDays(this.value.end, size + 1);

		return new DateRange({ start, end });
	}

	next() {
		if (isAfter(this.value.end, new Date())) return this;
		if (this.#value === "yesterday") return new DateRange("today");
		if (this.#isDayBeforeYesterday()) return new DateRange("yesterday");

		if (
			isEqual(startOfWeek(this.value.start), this.value.start) &&
			isEqual(endOfWeek(this.value.end), this.value.end) &&
			isSameWeek(this.value.start, this.value.end)
		) {
			const start = addWeeks(this.value.start, 1);
			const end = addWeeks(this.value.end, 1);
			return new DateRange({ start, end });
		}

		if (
			isEqual(startOfMonth(this.value.start), this.value.start) &&
			isEqual(endOfMonth(this.value.end), this.value.end) &&
			isSameMonth(this.value.start, this.value.end)
		) {
			const start = addMonths(this.value.start, 1);
			const end = addMonths(this.value.end, 1);
			return new DateRange({ start, end });
		}

		if (
			isEqual(startOfYear(this.value.start), this.value.start) &&
			isEqual(endOfYear(this.value.end), this.value.end) &&
			isSameYear(this.value.start, this.value.end)
		) {
			const start = addYears(this.value.start, 1);
			const end = addYears(this.value.end, 1);
			return new DateRange({ start, end });
		}

		if (differenceInHours(this.value.end, this.value.start) < 23) {
			const start = addSeconds(this.value.start, differenceInSeconds(this.value.end, this.value.start));
			const end = addSeconds(this.value.end, differenceInSeconds(this.value.end, this.value.start));
			return new DateRange({ start, end });
		}

		const size = differenceInDays(this.value.end, this.value.start);
		const start = addDays(this.value.start, size + 1);
		const end = addDays(this.value.end, size + 1);

		return new DateRange({ start, end });
	}
}

export const wellKnownRanges = {
	today: "Today",
	yesterday: "Yesterday",
	last7Days: "Last 7 Days",
	last30Days: "Last 30 Days",
	last12Months: "Last 12 Months",
	weekToDate: "Week to Date",
	monthToDate: "Month to Date",
	yearToDate: "Year to Date",
};
export type RangeName = keyof typeof wellKnownRanges;

const lastXDays = (days: number) => {
	const end = endOfDay(new Date());
	const start = startOfDay(subDays(end, days));
	return { start, end };
};

// all rangeNames are keys of the ranges object
export const ranges: Record<RangeName, () => { range: { start: Date; end: Date } }> = {
	today: () => {
		const now = new Date();
		const end = endOfDay(now);
		const start = startOfDay(now);
		return { range: { start, end } };
	},
	yesterday: () => {
		const now = new Date();
		const start = startOfDay(subDays(now, 1));
		const end = endOfDay(start);
		return { range: { start: start, end } };
	},
	last7Days: () => ({ range: lastXDays(7) }),
	last30Days: () => ({ range: lastXDays(30) }),
	last12Months: () => {
		const end = endOfMonth(new Date());
		const start = subMonths(end, 11);
		return { range: { start, end } };
	},
	weekToDate: () => {
		const now = new Date();
		const start = startOfWeek(now);
		const end = endOfWeek(now);
		return { range: { start, end } };
	},
	monthToDate: () => {
		const now = new Date();
		const start = startOfMonth(now);
		const end = endOfMonth(now);
		return { range: { start, end } };
	},
	yearToDate: () => {
		const now = new Date();
		const start = startOfYear(now);
		const end = endOfYear(now);
		return { range: { start, end: end } };
	},
};
