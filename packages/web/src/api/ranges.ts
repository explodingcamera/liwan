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
} from "date-fns";

import type { GraphInterval } from "@/constants";
import { formatDateRange } from "@/utils";

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
		const { start, end } = this.value;
		return `${this.serialize()}:${Number(start)}:${Number(end)}`;
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
		const dr = new DateRange({
			start: new Date(Number(start)),
			end: new Date(Number(end)),
		});
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

	#isDayBeforeYesterday() {
		return isSameDay(subDays(new Date(), 2), this.value.start) && isSameDay(subDays(new Date(), 2), this.value.end);
	}

	#shift(direction: -1 | 1): DateRange {
		const { start, end } = this.value;
		if (
			isEqual(startOfWeek(start, WEEK_STARTS_ON), start) &&
			isEqual(endOfWeek(end, WEEK_STARTS_ON), end) &&
			isSameWeek(start, end, WEEK_STARTS_ON)
		) {
			return new DateRange({ start: addWeeks(start, direction), end: addWeeks(end, direction) });
		}
		if (isEqual(startOfMonth(start), start) && isEqual(endOfMonth(end), end) && isSameMonth(start, end)) {
			return new DateRange({
				start: startOfMonth(addMonths(start, direction)),
				end: endOfMonth(addMonths(end, direction)),
			});
		}
		if (isEqual(startOfYear(start), start) && isEqual(endOfYear(end), end) && isSameYear(start, end)) {
			return new DateRange({
				start: startOfYear(addYears(start, direction)),
				end: endOfYear(addYears(end, direction)),
			});
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
