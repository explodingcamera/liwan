import {
	addDays,
	addHours,
	addMilliseconds,
	addMonths,
	addWeeks,
	addYears,
	differenceInCalendarDays,
	differenceInHours,
	differenceInMonths,
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
	subWeeks,
	subYears,
} from "date-fns";

import { formatDateRange } from "little-date";
import type { GraphInterval } from "./constants";
import type { GraphRange } from "../components/graph/graph";

type DateRangeValue = { start: Date; end: Date };
const WEEK_STARTS_ON = { weekStartsOn: 1 as const };

export class DateRange {
	#value: RangeName | { start: Date; end: Date };
	variant?: string;

	constructor(value: RangeName | { start: Date; end: Date }) {
		this.#value = value;
		if (typeof value === "string") this.variant = value;
	}

	get value(): DateRangeValue {
		if (typeof this.#value === "string") {
			return ranges[this.#value as RangeName]().range;
		}
		return this.#value as DateRangeValue;
	}

	isCustom(): boolean {
		return typeof this.#value !== "string" && !this.variant;
	}

	format(): string {
		if (this.variant === "allTime") return "All Time";
		if (typeof this.#value === "string") return wellKnownRanges[this.#value];
		return formatDateRange(this.#value.start, this.#value.end);
	}

	cacheKey(): string {
		return this.serialize();
	}

	serialize(): string {
		if (typeof this.#value === "string") return this.#value;
		return `${Number(this.#value.start)}:${Number(this.#value.end)}:${this.variant}`;
	}

	static deserialize(range: string): DateRange {
		if (!range.includes(":")) {
			return new DateRange(range as RangeName);
		}
		const [start, end, variant] = range.split(":");
		const dr = new DateRange({ start: new Date(Number(start)), end: new Date(Number(end)) });
		if (variant) {
			dr.variant = variant;
		}
		return dr;
	}

	endsToday(): boolean {
		// ends today or ends in the future
		return isEqual(endOfDay(new Date()), endOfDay(this.value.end)) || this.value.end > new Date();
	}

	getBucketBounds(): { start: Date; end: Date } {
		return {
			start: this.value.start,
			end: addMilliseconds(this.value.end, 1),
		};
	}

	#getDayCount(): number {
		return differenceInCalendarDays(this.value.end, this.value.start) + 1;
	}

	#isCalendarDayRange(): boolean {
		return isEqual(startOfDay(this.value.start), this.value.start) && isEqual(endOfDay(this.value.end), this.value.end);
	}

	#shiftByCalendarDays(direction: -1 | 1): DateRange {
		const dayCount = this.#getDayCount();
		return new DateRange({
			start: addDays(this.value.start, direction * dayCount),
			end: addDays(this.value.end, direction * dayCount),
		});
	}

	#shiftByExactDuration(direction: -1 | 1): DateRange {
		const { start, end } = this.getBucketBounds();
		const durationMs = end.getTime() - start.getTime();
		return new DateRange({
			start: new Date(this.value.start.getTime() + direction * durationMs),
			end: new Date(this.value.end.getTime() + direction * durationMs),
		});
	}

	#shiftByRange(direction: -1 | 1): DateRange {
		if (this.#isCalendarDayRange()) {
			return this.#shiftByCalendarDays(direction);
		}

		return this.#shiftByExactDuration(direction);
	}

	toAPI(): { start: string; end: string } {
		const { start, end } = this.getBucketBounds();
		const startIso = start.toISOString();
		const endIso = end.toISOString();
		return { start: startIso, end: endIso };
	}

	getGraphRange(): GraphRange {
		return this.getGraphInterval();
	}

	getGraphInterval(): GraphInterval {
		if (this.variant === "last7DaysHourly") return "hour";
		if (this.variant === "weekToDate") return this.#getDayCount() < 4 ? "hour" : "day";
		if (this.variant === "monthToDate") return this.#getDayCount() < 7 ? "hour" : "day";
		if (this.#getDayCount() < 7) return "hour";
		return "day";
	}

	getGraphBucketEnd(bucketStart: Date): Date {
		const bucketEnd = this.getGraphInterval() === "hour" ? addHours(bucketStart, 1) : addDays(bucketStart, 1);
		const { end } = this.getBucketBounds();
		return bucketEnd < end ? bucketEnd : end;
	}

	getAxisRange(): "hour" | "day" | "day+year" {
		const { end } = this.getBucketBounds();
		if (differenceInHours(end, this.value.start) <= 24) return "hour";
		if (differenceInYears(this.value.end, addHours(this.value.start, 1)) > 1) return "day+year";
		return "day";
	}

	getTooltipRange(): "hour" | "day+hour" | "day" {
		if (this.getGraphInterval() === "day") return "day";
		const dayCount = this.#getDayCount();
		if (dayCount === 1) return "hour";
		return "day+hour";
	}

	#isDayBeforeYesterday() {
		return isSameDay(subDays(new Date(), 2), this.value.start) && isSameDay(subDays(new Date(), 2), this.value.end);
	}

	previous() {
		if (this.variant === "allTime") return this;
		if (this.#value === "today") return new DateRange("yesterday");

		if (
			isEqual(startOfWeek(this.value.start, WEEK_STARTS_ON), this.value.start) &&
			isEqual(endOfWeek(this.value.end, WEEK_STARTS_ON), this.value.end) &&
			isSameWeek(this.value.start, this.value.end, WEEK_STARTS_ON)
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

		if (differenceInMonths(this.value.end, this.value.start) === 12) {
			const start = subYears(this.value.start, 1);
			const end = subYears(this.value.end, 1);
			return new DateRange({ start, end });
		}

		return this.#shiftByRange(-1);
	}

	next() {
		if (isAfter(this.value.end, new Date())) return this;
		if (this.#value === "yesterday") return new DateRange("today");
		if (this.#isDayBeforeYesterday()) return new DateRange("yesterday");

		if (
			isEqual(startOfWeek(this.value.start, WEEK_STARTS_ON), this.value.start) &&
			isEqual(endOfWeek(this.value.end, WEEK_STARTS_ON), this.value.end) &&
			isSameWeek(this.value.start, this.value.end, WEEK_STARTS_ON)
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

		if (differenceInMonths(this.value.end, this.value.start) === 12) {
			const start = addYears(this.value.start, 1);
			const end = addYears(this.value.end, 1);
			return new DateRange({ start, end });
		}

		return this.#shiftByRange(1);
	}
}

export const wellKnownRanges = {
	today: "Today",
	yesterday: "Yesterday",
	last7DaysHourly: "Last 7 Days (hourly)",
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
	const start = startOfDay(subDays(end, days - 1));
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
	last7DaysHourly: () => ({ range: lastXDays(7) }),
	last7Days: () => ({ range: lastXDays(7) }),
	last30Days: () => ({ range: lastXDays(30) }),
	last12Months: () => {
		const start = startOfMonth(subYears(new Date(), 1));
		const end = endOfMonth(new Date());
		return { range: { start, end } };
	},
	weekToDate: () => {
		const now = new Date();
		const start = startOfWeek(now, WEEK_STARTS_ON);
		const end = endOfDay(now);
		return { range: { start, end } };
	},
	monthToDate: () => {
		const now = new Date();
		const start = startOfMonth(now);
		const end = endOfDay(now);
		return { range: { start, end } };
	},
	yearToDate: () => {
		const now = new Date();
		const start = startOfYear(now);
		const end = endOfDay(now);
		return { range: { start, end: end } };
	},
};
