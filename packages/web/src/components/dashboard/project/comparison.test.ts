import { describe, expect, test } from "bun:test";
import { endOfDay } from "date-fns";

import { DateRange } from "@/api/ranges";
import {
	ComparisonRange,
	getComparisonOptions,
	getComparisonRange,
	getElapsedComparisonRange,
	getStatsReportRanges,
	mergeComparisonRows,
} from "./comparison";

const range = (start: Date, end = start) => new DateRange({ start, end: endOfDay(end) });

describe("comparison periods", () => {
	test("matches the selected duration from a custom start date", () => {
		const custom = { mode: "custom", start: new Date(2025, 10, 5) } as const;
		expect(getComparisonRange(range(new Date(2026, 10, 1), new Date(2026, 10, 5)), custom)?.value).toEqual(
			range(new Date(2025, 10, 5), new Date(2025, 10, 9)).value,
		);
		expect(getComparisonRange(range(new Date(2026, 10, 1), new Date(2026, 10, 7)), custom)?.value.end).toEqual(
			endOfDay(new Date(2025, 10, 11)),
		);
	});

	test("uses an adjacent equal-length period", () => {
		const current = range(new Date(2026, 10, 1), new Date(2026, 10, 5));
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual(
			range(new Date(2026, 9, 27), new Date(2026, 9, 31)).value,
		);
	});

	test("does not compare all time", () => {
		const current = range(new Date(2026, 10, 1), new Date(2026, 10, 5));
		current.variant = "allTime";
		expect(getComparisonRange(current, { mode: "previous" })).toBeUndefined();
		expect(getComparisonOptions(current)).toEqual([]);
	});

	test("compares whole months from the first through the last day", () => {
		const current = range(new Date(2025, 2, 1), new Date(2025, 2, 31));
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual(
			range(new Date(2025, 1, 1), new Date(2025, 1, 28)).value,
		);
		expect(getComparisonRange(current, { mode: "custom", start: new Date(2025, 3, 1) })?.value).toEqual(
			range(new Date(2025, 3, 1), new Date(2025, 3, 30)).value,
		);
	});

	test("keeps rolling day ranges adjacent even at month boundaries", () => {
		const current = range(new Date(2025, 3, 1), new Date(2025, 3, 30));
		current.variant = "last30Days";
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual(
			range(new Date(2025, 2, 2), new Date(2025, 2, 31)).value,
		);
	});

	test("compares month to date with the full previous month", () => {
		const current = range(new Date(2025, 2, 1), new Date(2025, 2, 15));
		current.variant = "monthToDate";
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual(
			range(new Date(2025, 1, 1), new Date(2025, 1, 28)).value,
		);
	});

	test("compares week to date with the full previous week", () => {
		const current = range(new Date(2026, 10, 2), new Date(2026, 10, 5));
		current.variant = "weekToDate";
		expect(getComparisonRange(current, { mode: "week" })?.value).toEqual(
			range(new Date(2026, 9, 26), new Date(2026, 10, 1)).value,
		);
	});

	test("shifts weekly comparisons by calendar days across DST", () => {
		const current = range(new Date(2026, 2, 8));
		const previous = getComparisonRange(current, { mode: "week" });
		expect(previous?.value).toEqual(range(new Date(2026, 2, 1)).value);
		expect(previous && getElapsedComparisonRange(current, previous, new Date(2026, 2, 8, 12)).value.end).toEqual(
			new Date(2026, 2, 1, 12),
		);
	});

	test("shifts intraday selections by their duration", () => {
		const current = new DateRange({ start: new Date(2026, 10, 5, 10), end: new Date(2026, 10, 5, 11, 59, 59, 999) });
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual({
			start: new Date(2026, 10, 5, 8),
			end: new Date(2026, 10, 5, 9, 59, 59, 999),
		});
	});

	test("handles leap days for last-year comparisons", () => {
		expect(getComparisonRange(range(new Date(2024, 1, 29)), { mode: "year" })?.value).toEqual(
			range(new Date(2023, 1, 28)).value,
		);
	});

	test("clips ongoing periods to equivalent elapsed time", () => {
		const current = range(new Date(2026, 10, 1), new Date(2026, 10, 5));
		const previous = new ComparisonRange(range(new Date(2026, 9, 27), new Date(2026, 9, 31)).value, "day", -5);
		expect(getElapsedComparisonRange(current, previous, new Date(2026, 10, 5, 12, 30)).value.end).toEqual(
			new Date(2026, 9, 31, 12, 30),
		);
		expect(getElapsedComparisonRange(current, previous, new Date(2026, 10, 6))).toBe(previous);
	});

	test("does not extend a shorter custom period", () => {
		const previous = new ComparisonRange(range(new Date(2026, 9, 1)).value, "day", -31);
		expect(
			getElapsedComparisonRange(
				range(new Date(2026, 10, 1), new Date(2026, 10, 5)),
				previous,
				new Date(2026, 10, 5, 12),
			).value.end,
		).toEqual(previous.value.end);
	});

	test("preserves local clock times across daylight-saving changes", () => {
		const current = range(new Date(2026, 2, 8));
		const previous = getComparisonRange(current, { mode: "previous" });
		expect(previous?.value).toEqual(range(new Date(2026, 2, 7)).value);
		expect(previous && getElapsedComparisonRange(current, previous, new Date(2026, 2, 8, 12)).value.end).toEqual(
			new Date(2026, 2, 7, 12),
		);
	});

	test("keeps last-year cutoffs on the same date after leap day", () => {
		const current = range(new Date(2024, 0, 1), new Date(2024, 11, 31));
		const previous = new ComparisonRange(range(new Date(2023, 0, 1), new Date(2023, 11, 31)).value, "year", -1);
		expect(getElapsedComparisonRange(current, previous, new Date(2024, 2, 1, 12)).value.end).toEqual(
			new Date(2023, 2, 1, 12),
		);
	});

	test("compares year to date with the full previous year", () => {
		const current = range(new Date(2024, 0, 1), new Date(2024, 2, 15));
		current.variant = "yearToDate";
		const previous = getComparisonRange(current, { mode: "year" });
		expect(previous?.value).toEqual(range(new Date(2023, 0, 1), new Date(2023, 11, 31)).value);
		expect(previous && getElapsedComparisonRange(current, previous, new Date(2024, 2, 1, 12))).toBe(previous);
	});

	test("keeps the whole comparison month when it has fewer days", () => {
		const current = range(new Date(2025, 2, 1), new Date(2025, 2, 31));
		const previous = getComparisonRange(current, { mode: "previous" });
		expect(previous && getElapsedComparisonRange(current, previous, new Date(2025, 2, 30, 12)).value.end).toEqual(
			endOfDay(new Date(2025, 1, 28)),
		);
	});
});

test("stats use the preceding span by default but not for all time", () => {
	const current = new DateRange({ start: new Date(2026, 10, 5, 10), end: new Date(2026, 10, 5, 11, 59, 59, 999) });
	expect(getStatsReportRanges(current)).toEqual([
		current.toAPI(),
		{ start: new Date(2026, 10, 5, 8).toISOString(), end: current.value.start.toISOString() },
	]);

	const comparison = getComparisonRange(current, { mode: "year" });
	const lastYear = { start: new Date(2025, 10, 5, 10).toISOString(), end: new Date(2025, 10, 5, 12).toISOString() };
	expect(getStatsReportRanges(current, comparison)).toEqual([current.toAPI(), lastYear]);

	current.variant = "allTime";
	expect(getStatsReportRanges(current)).toEqual([current.toAPI()]);
});

describe("comparison options", () => {
	test("offers the previous year once for year to date", () => {
		const current = range(new Date(2024, 0, 1), new Date(2024, 2, 15));
		current.variant = "yearToDate";
		expect(getComparisonOptions(current).map(({ mode }) => mode)).toEqual(["previous"]);
		expect(getComparisonRange(current, { mode: "previous" })?.value).toEqual(
			range(new Date(2023, 0, 1), new Date(2023, 11, 31)).value,
		);
	});

	test("does not duplicate the preceding week", () => {
		const current = range(new Date(2026, 9, 30), new Date(2026, 10, 5));
		current.variant = "last7Days";
		expect(getComparisonOptions(current).map(({ mode }) => mode)).toEqual(["previous", "year"]);
		const weekToDate = range(new Date(2026, 10, 2), new Date(2026, 10, 5));
		weekToDate.variant = "weekToDate";
		expect(getComparisonOptions(weekToDate).map(({ mode }) => mode)).toEqual(["previous", "year"]);
	});

	test("keeps month-to-date options independent of the day of the month", () => {
		for (const day of [1, 5, 15]) {
			const current = range(new Date(2026, 10, 1), new Date(2026, 10, day));
			current.variant = "monthToDate";
			expect(getComparisonOptions(current).map(({ mode }) => mode)).toEqual(["previous", "year"]);
		}
	});

	test("replaces an incompatible weekly comparison with the range's previous period", () => {
		const current = range(new Date(2026, 10, 1), new Date(2026, 10, 5));
		current.variant = "monthToDate";
		expect(getComparisonRange(current, { mode: "week" })?.value).toEqual(
			getComparisonRange(current, { mode: "previous" })?.value,
		);
	});
});

test("preserves unavailable and zero comparison values", () => {
	const current = [
		{ dimensionValue: "a", value: 5 },
		{ dimensionValue: "b", value: null },
		{ dimensionValue: "c", value: 0 },
	];
	expect(
		mergeComparisonRows(current, [
			{ dimensionValue: "a", value: 0 },
			{ dimensionValue: "b", value: 3 },
			{ dimensionValue: "c", value: null },
		]),
	).toEqual([
		{ dimensionValue: "a", value: 5, comparisonValue: 0 },
		{ dimensionValue: "b", value: null, comparisonValue: 3 },
		{ dimensionValue: "c", value: 0, comparisonValue: null },
	]);
	expect(mergeComparisonRows(current)).toEqual(current);
});
