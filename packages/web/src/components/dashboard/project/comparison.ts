import {
	addDays,
	addMonths,
	addYears,
	differenceInCalendarDays,
	differenceInCalendarMonths,
	endOfDay,
	endOfMonth,
	endOfWeek,
	endOfYear,
	isEqual,
	startOfDay,
	startOfMonth,
} from "date-fns";

import { DateRange } from "@/api/ranges";
import type { DimensionTableRow } from "@/constants";

export type ComparisonPreset = "previous" | "week" | "year";
export type Comparison = { mode: "off" | ComparisonPreset } | { mode: "custom"; start: Date };
export type ComparisonRow = DimensionTableRow & { comparisonValue?: number | null };

/** A resolved period with the calendar offset used for ongoing-period cutoffs. */
export class ComparisonRange extends DateRange {
	constructor(
		value: { start: Date; end: Date },
		readonly offsetUnit: "year" | "month" | "day" | "duration",
		readonly offset: number,
		readonly fullPeriod = false,
	) {
		super(value);
	}

	/** Map a date in the current period to the corresponding comparison date. */
	mapDate(date: Date): Date {
		switch (this.offsetUnit) {
			case "year":
				return addYears(date, this.offset);
			case "month":
				return addMonths(date, this.offset);
			case "day":
				return addDays(date, this.offset);
			case "duration":
				return new Date(date.getTime() + this.offset);
		}
	}

	cacheKey(): string {
		return `${super.cacheKey()}:${this.offsetUnit}:${this.offset}:${this.fullPeriod}`;
	}
}

export const getComparisonRange = (range: DateRange, comparison: Comparison): ComparisonRange | undefined => {
	if (comparison.mode === "off" || range.variant === "allTime") return;
	const { start, end } = range.value;
	if (comparison.mode === "week") {
		const canCompareWeek = range.variant
			? ["today", "yesterday", "last7Days", "weekToDate"].includes(range.variant)
			: differenceInCalendarDays(end, start) < 7;
		if (!canCompareWeek) return getComparisonRange(range, { mode: "previous" });
	}
	if (range.variant === "weekToDate" && (comparison.mode === "previous" || comparison.mode === "week")) {
		const previous = addDays(start, -7);
		return new ComparisonRange({ start: previous, end: endOfWeek(previous, { weekStartsOn: 1 }) }, "day", -7, true);
	}
	if (range.variant === "monthToDate" && comparison.mode === "previous") {
		const previous = addMonths(start, -1);
		return new ComparisonRange({ start: previous, end: endOfMonth(previous) }, "month", -1, true);
	}
	if (range.variant === "yearToDate" && (comparison.mode === "previous" || comparison.mode === "year")) {
		const previous = addYears(start, -1);
		return new ComparisonRange({ start: previous, end: endOfYear(previous) }, "year", -1, true);
	}
	const calendarDays = isEqual(start, startOfDay(start)) && isEqual(end, endOfDay(end));
	const wholeMonths =
		isEqual(start, startOfMonth(start)) &&
		isEqual(end, endOfMonth(end)) &&
		!["last7Days", "last30Days"].includes(range.variant ?? "");
	const monthCount = differenceInCalendarMonths(end, start) + 1;
	const calendarMonths = wholeMonths || ["monthToDate", "yearToDate", "last12Months"].includes(range.variant ?? "");
	if (comparison.mode === "year")
		return new ComparisonRange({ start: addYears(start, -1), end: addYears(end, -1) }, "year", -1);
	if (comparison.mode === "week")
		return new ComparisonRange({ start: addDays(start, -7), end: addDays(end, -7) }, "day", -7);
	if (comparison.mode === "custom") {
		const customStart = new Date(comparison.start);
		customStart.setHours(start.getHours(), start.getMinutes(), start.getSeconds(), start.getMilliseconds());
		if (calendarMonths && customStart.getDate() === 1) {
			const offset = differenceInCalendarMonths(customStart, start);
			return new ComparisonRange(
				{
					start: customStart,
					end: wholeMonths ? endOfMonth(addMonths(customStart, monthCount - 1)) : addMonths(end, offset),
				},
				"month",
				offset,
			);
		}
		return new ComparisonRange(
			{
				start: customStart,
				end: calendarDays
					? endOfDay(addDays(customStart, differenceInCalendarDays(end, start)))
					: new Date(customStart.getTime() + end.getTime() - start.getTime()),
			},
			calendarDays ? "day" : "duration",
			calendarDays ? differenceInCalendarDays(customStart, start) : customStart.getTime() - start.getTime(),
		);
	}
	if (wholeMonths || range.variant === "last12Months") {
		const offset = wholeMonths ? -monthCount : -12;
		return new ComparisonRange(
			{ start: addMonths(start, offset), end: wholeMonths ? endOfMonth(addMonths(start, -1)) : addMonths(end, offset) },
			"month",
			offset,
		);
	}
	if (!calendarDays) {
		const duration = end.getTime() - start.getTime() + 1;
		return new ComparisonRange(
			{ start: new Date(start.getTime() - duration), end: new Date(start.getTime() - 1) },
			"duration",
			-duration,
		);
	}
	// Use calendar days so a daylight-saving change does not shift the time of day.
	const days = differenceInCalendarDays(end, start) + 1;
	return new ComparisonRange({ start: addDays(start, -days), end: addDays(end, -days) }, "day", -days);
};

/** Offer presets based on the selected range's meaning, without duplicate dates. */
export const getComparisonOptions = (range: DateRange) => {
	const labels = {
		previous: "Previous period",
		week: "Last week",
		year: "Last year",
	};
	const modes = ["previous", "week", "year"] as const;
	const seen = new Set<string>();
	return modes.flatMap((mode) => {
		const comparisonRange = getComparisonRange(range, { mode });
		if (!comparisonRange) return [];
		const { start, end } = comparisonRange.value;
		const key = `${start.getTime()}:${end.getTime()}`;
		if (seen.has(key)) return [];
		seen.add(key);
		return [{ mode, label: labels[mode], range: comparisonRange }];
	});
};

// Clip matched comparisons to equivalent elapsed time, but keep full calendar comparisons intact.
export const getElapsedComparisonRange = (
	range: DateRange,
	comparison: ComparisonRange,
	now = new Date(),
): DateRange => {
	const current = range.value;
	const previous = comparison.value;
	if (comparison.fullPeriod || now < current.start || now >= current.end) return comparison;
	const mapped = comparison.mapDate(now);
	// A shorter comparison month is complete once the current date passes its last day.
	const end =
		(comparison.offsetUnit === "month" || comparison.offsetUnit === "year") && mapped.getDate() !== now.getDate()
			? endOfDay(mapped)
			: mapped;
	return new DateRange({ start: previous.start, end: new Date(Math.min(previous.end.getTime(), end.getTime())) });
};

/** Build the requested periods, clipping an ongoing comparison to its elapsed time. */
export const getReportRanges = (range: DateRange, comparison?: ComparisonRange) => {
	const ranges = [range.toAPI()];
	if (comparison) ranges.push(getElapsedComparisonRange(range, comparison).toAPI());
	return ranges;
};

/** Summary stats compare with the immediately preceding span when no comparison is selected. */
export const getStatsReportRanges = (range: DateRange, comparison?: ComparisonRange) => {
	if (range.variant === "allTime") return [range.toAPI()];
	if (comparison) return getReportRanges(range, comparison);

	const current = range.toAPI();
	const start = Date.parse(current.start);
	const duration = Date.parse(current.end) - start;
	return [current, { start: new Date(start - duration).toISOString(), end: current.start }];
};

export const mergeComparisonRows = (current: DimensionTableRow[], previous?: DimensionTableRow[]): ComparisonRow[] => {
	if (!previous) return current;
	const values = new Map(previous.map((row) => [row.dimensionValue, row.value]));
	return current.map((row) => ({ ...row, comparisonValue: values.get(row.dimensionValue) ?? null }));
};
