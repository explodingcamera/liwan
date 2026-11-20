import {
	addDays,
	addHours,
	addMilliseconds,
	addMonths,
	addWeeks,
	addYears,
	differenceInCalendarDays,
	differenceInMonths,
	endOfDay,
	endOfMonth,
	endOfWeek,
	endOfYear,
	getISOWeek,
	getISOWeekYear,
	isAfter,
	isEqual,
	isSameDay,
	isSameMonth,
	isSameWeek,
	isSameYear,
	min,
	startOfDay,
	startOfMonth,
	startOfWeek,
	startOfYear,
	subDays,
	subMonths,
} from "date-fns";

import type { GraphInterval } from "@/constants";
import { formatDateRange } from "@/utils";

type DateRangeValue = { start: Date; end: Date };
const WEEK_STARTS_ON = { weekStartsOn: 1 as const };
const isFullWeek = (start: Date, end: Date) =>
	isEqual(startOfWeek(start, WEEK_STARTS_ON), start) && isEqual(endOfWeek(start, WEEK_STARTS_ON), end);

export class DateRange {
	#value: RangeName | DateRangeValue;
	variant?: RangeName | "allTime";

	constructor(value: RangeName | DateRangeValue) {
		this.#value = value;
		if (typeof value === "string") this.variant = value;
	}

	get value(): DateRangeValue {
		if (typeof this.#value === "string") {
			return ranges[this.#value]().range;
		}
		return this.#value;
	}

	isCustom(): boolean {
		return typeof this.#value !== "string" && !this.variant;
	}

	format(): string {
		if (this.variant === "allTime") return "All Time";
		if (typeof this.#value === "string") return wellKnownRanges[this.#value];
		const { start, end } = this.#value;
		if (isFullWeek(start, end)) {
			const week = `Week ${getISOWeek(start)}`;
			const year = getISOWeekYear(start);
			return year === getISOWeekYear(new Date()) ? week : `${week}, ${year}`;
		}
		return formatDateRange(start, end);
	}

	cacheKey(): string {
		const { start, end } = this.value;
		return `${this.serialize()}:${Number(start)}:${Number(end)}`;
	}

	serialize(): string {
		if (typeof this.#value === "string") return this.#value;
		return `${Number(this.#value.start)}:${Number(this.#value.end)}:${this.variant ?? ""}`;
	}

	static deserialize(range: string): DateRange {
		if (!range.includes(":")) {
			if (!isRangeName(range)) throw new Error("Invalid date range preset");
			return new DateRange(range);
		}
		const [start, end, variant] = range.split(":");
		const from = new Date(Number(start));
		const through = new Date(Number(end));
		if (!start || !end || Number.isNaN(+from) || Number.isNaN(+through) || from > through) {
			throw new Error("Invalid date range");
		}
		const dr = new DateRange({ start: from, end: through });
		if (variant === "allTime" || (variant && isRangeName(variant))) {
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

	#isRollingYearRange(): boolean {
		return (
			[11, 12].includes(differenceInMonths(this.value.end, this.value.start)) &&
			isEqual(startOfMonth(this.value.start), this.value.start) &&
			isEqual(endOfDay(this.value.end), this.value.end)
		);
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

	toAPI(): { start: string; end: string } {
		const { start, end } = this.getBucketBounds();
		const startIso = start.toISOString();
		const endIso = end.toISOString();
		return { start: startIso, end: endIso };
	}

	getGraphInterval(): GraphInterval {
		const days = this.#getDayCount();
		if (this.variant === "weekToDate") return days < 4 ? "hour" : "day";
		if (this.variant === "monthToDate") return days < 7 ? "hour" : "day";
		if (days < 7) return "hour";
		if (days > 160 * 7) return "month";
		if (days > 366) return "week";
		return "day";
	}

	getGraphIntervals(): GraphInterval[] {
		const days = this.#getDayCount();
		return [
			...(days <= 30 ? ["hour" as const] : []),
			...(days >= 2 && days <= 2 * 366 ? ["day" as const] : []),
			...(days >= 14 ? ["week" as const] : []),
			...(days >= 60 ? ["month" as const] : []),
		];
	}

	getGraphBucketEnd(bucketStart: Date, interval: GraphInterval = this.getGraphInterval()): Date {
		const bucketEnd =
			interval === "hour"
				? addHours(bucketStart, 1)
				: interval === "week"
					? addWeeks(bucketStart, 1)
					: interval === "month"
						? addMonths(bucketStart, 1)
						: addDays(bucketStart, 1);
		const { end } = this.getBucketBounds();
		return min([bucketEnd, end]);
	}

	#isDayBeforeYesterday() {
		return isSameDay(subDays(new Date(), 2), this.value.start) && isSameDay(subDays(new Date(), 2), this.value.end);
	}

	#shift(direction: -1 | 1): DateRange {
		const { start, end } = this.value;
		if (this.#value === "weekToDate" || isFullWeek(start, end)) {
			const shifted = addWeeks(start, direction);
			if (direction === 1 && isSameWeek(shifted, new Date(), WEEK_STARTS_ON)) return new DateRange("weekToDate");
			return new DateRange({ start: shifted, end: endOfWeek(shifted, WEEK_STARTS_ON) });
		}
		if (
			this.#value === "monthToDate" ||
			(isEqual(startOfMonth(start), start) && isEqual(endOfMonth(end), end) && isSameMonth(start, end))
		) {
			const shifted = addMonths(start, direction);
			if (direction === 1 && isSameMonth(shifted, new Date())) return new DateRange("monthToDate");
			return new DateRange({ start: shifted, end: endOfMonth(shifted) });
		}
		if (
			this.#value === "yearToDate" ||
			(isEqual(startOfYear(start), start) && isEqual(endOfYear(end), end) && isSameYear(start, end))
		) {
			const shifted = addYears(start, direction);
			if (direction === 1 && isSameYear(shifted, new Date())) return new DateRange("yearToDate");
			return new DateRange({ start: shifted, end: endOfYear(shifted) });
		}
		if (this.#isRollingYearRange()) {
			return new DateRange({ start: addYears(start, direction), end: addYears(end, direction) });
		}
		return this.#isCalendarDayRange() ? this.#shiftByCalendarDays(direction) : this.#shiftByExactDuration(direction);
	}

	previous() {
		if (this.variant === "allTime") return this;
		if (this.#value === "today") return new DateRange("yesterday");
		return this.#shift(-1);
	}

	next() {
		if (isAfter(this.value.end, new Date())) return this;
		if (this.#value === "yesterday") return new DateRange("today");
		if (this.#isDayBeforeYesterday()) return new DateRange("yesterday");
		return this.#shift(1);
	}
}

export const wellKnownRanges = {
	today: "Today",
	yesterday: "Yesterday",
	weekToDate: "Week to Date",
	monthToDate: "Month to Date",
	yearToDate: "Year to Date",
	last7Days: "Last 7 Days",
	last30Days: "Last 30 Days",
	last12Months: "Last 12 Months",
};
export type RangeName = keyof typeof wellKnownRanges;

/** Check whether a string names a date range preset. */
export const isRangeName = (value: string): value is RangeName => Object.hasOwn(wellKnownRanges, value);

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
	last7Days: () => ({ range: lastXDays(7) }),
	last30Days: () => ({ range: lastXDays(30) }),
	last12Months: () => {
		const now = new Date();
		const start = startOfMonth(subMonths(now, 11));
		const end = endOfDay(now);
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
