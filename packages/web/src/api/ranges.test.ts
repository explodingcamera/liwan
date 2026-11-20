import { describe, expect, it } from "bun:test";
import {
	differenceInCalendarDays,
	endOfDay,
	endOfMonth,
	endOfWeek,
	endOfYear,
	getISOWeekYear,
	startOfDay,
	startOfMonth,
	startOfWeek,
	startOfYear,
	subDays,
	subMonths,
	subWeeks,
	subYears,
} from "date-fns";

import { DateRange, ranges } from "./ranges";

describe("DateRange", () => {
	it("should initialize with a range name", () => {
		const range = new DateRange("today");
		expect(range.isCustom()).toBe(false);
		expect(range.serialize()).toBe("today");
	});

	it("should initialize with a custom date range", () => {
		const start = new Date(2024, 10, 1);
		const end = new Date(2024, 10, 15);
		const range = new DateRange({ start, end });
		expect(range.isCustom()).toBe(true);
		expect(range.value).toEqual({ start, end });
	});

	it("should serialize and deserialize correctly", () => {
		const start = new Date(2024, 10, 1);
		const end = new Date(2024, 10, 15);
		const range = new DateRange({ start, end });
		const serialized = range.serialize();
		const deserialized = DateRange.deserialize(serialized);
		expect(deserialized.value).toEqual({ start, end });
		expect(deserialized.isCustom()).toBe(true);
		expect(DateRange.deserialize(`${serialized}undefined`).isCustom()).toBe(true);
	});

	it("should persist named ranges dynamically but cache by resolved dates", () => {
		const range = new DateRange("last30Days");

		expect(range.serialize()).toBe("last30Days");
		expect(range.cacheKey()).toContain("last30Days:");
	});

	it("should format well-known ranges", () => {
		const range = new DateRange("today");
		expect(range.format()).toBeDefined(); // Ensure a format exists
	});

	it("labels full Monday-to-Sunday ranges by ISO week", () => {
		const yearStart = new Date(getISOWeekYear(new Date()), 0, 4);
		const weekStart = startOfWeek(yearStart, { weekStartsOn: 1 });
		const weekEnd = endOfWeek(yearStart, { weekStartsOn: 1 });
		expect(new DateRange({ start: weekStart, end: weekEnd }).format()).toBe("Week 1");

		const start = new Date(2019, 11, 30);
		expect(new DateRange({ start, end: endOfDay(new Date(2020, 0, 5)) }).format()).toBe("Week 1, 2020");
		expect(new DateRange({ start, end: endOfDay(new Date(2020, 0, 4)) }).format()).not.toMatch(/^Week /);
	});

	it("should calculate if the range ends today", () => {
		const now = startOfDay(new Date());
		const range = new DateRange({ start: now, end: now });
		expect(range.endsToday()).toBe(true);

		const yesterday = subDays(now, 1);
		const pastRange = new DateRange({ start: yesterday, end: yesterday });
		expect(pastRange.endsToday()).toBe(false);
	});

	it("should calculate graph interval", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 7));
		const range = new DateRange({ start, end });
		expect(range.getGraphInterval()).toBe("day");
	});

	it("offers intervals suited to the selected range", () => {
		const rangeForDays = (days: number) =>
			new DateRange({ start: new Date(2024, 0, 1), end: endOfDay(new Date(2024, 0, days)) });

		expect(rangeForDays(1).getGraphIntervals()).toEqual(["hour"]);
		expect(rangeForDays(7).getGraphIntervals()).toEqual(["hour", "day"]);
		expect(rangeForDays(30).getGraphIntervals()).toEqual(["hour", "day", "week"]);
		expect(rangeForDays(90).getGraphIntervals()).toEqual(["day", "week", "month"]);
		expect(rangeForDays(400).getGraphIntervals()).toEqual(["day", "week", "month"]);
		expect(rangeForDays(732).getGraphIntervals()).toEqual(["day", "week", "month"]);
		expect(rangeForDays(733).getGraphIntervals()).toEqual(["week", "month"]);
		expect(rangeForDays(6).getGraphInterval()).toBe("hour");
		expect(rangeForDays(30).getGraphInterval()).toBe("day");
		expect(rangeForDays(90).getGraphInterval()).toBe("day");
		expect(rangeForDays(180).getGraphInterval()).toBe("day");
		expect(rangeForDays(366).getGraphInterval()).toBe("day");
		expect(rangeForDays(367).getGraphInterval()).toBe("week");
		expect(rangeForDays(400).getGraphInterval()).toBe("week");
		expect(rangeForDays(1120).getGraphInterval()).toBe("week");
		expect(rangeForDays(1121).getGraphInterval()).toBe("month");
	});

	it("should use inclusive calendar hours for short multi-day ranges", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 2));
		const range = new DateRange({ start, end });

		expect(range.getGraphInterval()).toBe("hour");
	});

	it("should send an exclusive end boundary to the API", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 7));
		const range = new DateRange({ start, end });

		expect(range.toAPI()).toEqual({
			start: start.toISOString(),
			end: new Date(end.getTime() + 1).toISOString(),
		});
	});

	it("should keep last7Days at seven calendar days", () => {
		const { start, end } = ranges.last7Days().range;

		expect(differenceInCalendarDays(end, start) + 1).toBe(7);
	});

	it("should keep last12Months to the current month and never future months", () => {
		const now = new Date();
		const { start, end } = ranges.last12Months().range;

		expect(start).toEqual(startOfMonth(subMonths(now, 11)));
		expect(end).toEqual(endOfDay(now));
	});

	it("should start weekToDate on monday and end today", () => {
		const { start, end } = ranges.weekToDate().range;

		expect(start.getDay()).toBe(1);
		expect(end.getDate()).toBe(new Date().getDate());
		expect(end.getMonth()).toBe(new Date().getMonth());
		expect(end.getFullYear()).toBe(new Date().getFullYear());
	});

	it("should end monthToDate today", () => {
		const { end } = ranges.monthToDate().range;

		expect(end.getDate()).toBe(new Date().getDate());
		expect(end.getMonth()).toBe(new Date().getMonth());
		expect(end.getFullYear()).toBe(new Date().getFullYear());
	});

	it("should use variant-specific hourly cutovers for week and month to date", () => {
		const weekShort = new DateRange({
			start: startOfDay(new Date(2024, 10, 1)),
			end: endOfDay(new Date(2024, 10, 3)),
		});
		weekShort.variant = "weekToDate";
		expect(weekShort.getGraphInterval()).toBe("hour");

		const weekLong = new DateRange({
			start: startOfDay(new Date(2024, 10, 1)),
			end: endOfDay(new Date(2024, 10, 4)),
		});
		weekLong.variant = "weekToDate";
		expect(weekLong.getGraphInterval()).toBe("day");

		const monthShort = new DateRange({
			start: startOfDay(new Date(2024, 10, 1)),
			end: endOfDay(new Date(2024, 10, 6)),
		});
		monthShort.variant = "monthToDate";
		expect(monthShort.getGraphInterval()).toBe("hour");

		const monthLong = new DateRange({
			start: startOfDay(new Date(2024, 10, 1)),
			end: endOfDay(new Date(2024, 10, 7)),
		});
		monthLong.variant = "monthToDate";
		expect(monthLong.getGraphInterval()).toBe("day");
	});

	it("should cap the trailing graph bucket at the selected range end", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = new Date(2024, 10, 1, 1, 0, 0);
		const range = new DateRange({ start, end });

		expect(range.getGraphBucketEnd(start)).toEqual(new Date(2024, 10, 1, 1, 0, 0));
	});

	it("should shift custom calendar ranges by the full selected span", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 3));
		const range = new DateRange({ start, end });

		expect(range.previous().value).toEqual({
			start: startOfDay(new Date(2024, 9, 29)),
			end: endOfDay(new Date(2024, 9, 31)),
		});
		expect(range.next().value).toEqual({
			start: startOfDay(new Date(2024, 10, 4)),
			end: endOfDay(new Date(2024, 10, 6)),
		});
	});

	it("should shift full months to the full next month", () => {
		const range = new DateRange({ start: startOfMonth(new Date(2023, 1, 1)), end: endOfMonth(new Date(2023, 1, 1)) });

		expect(range.next().value).toEqual({
			start: startOfMonth(new Date(2023, 2, 1)),
			end: endOfMonth(new Date(2023, 2, 1)),
		});
	});

	it("navigates to full past periods and back to the current period to date", () => {
		const now = new Date();
		const weekOptions = { weekStartsOn: 1 } as const;
		const lastWeek = subWeeks(now, 1);
		const lastMonth = subMonths(now, 1);
		const lastYear = subYears(now, 1);
		const periods = [
			["weekToDate", startOfWeek(lastWeek, weekOptions), endOfWeek(lastWeek, weekOptions)],
			["monthToDate", startOfMonth(lastMonth), endOfMonth(lastMonth)],
			["yearToDate", startOfYear(lastYear), endOfYear(lastYear)],
		] as const;

		for (const [variant, start, end] of periods) {
			const current = new DateRange(variant);
			const previous = current.previous();
			expect(previous.value).toEqual({ start, end });
			expect(previous.previous().next().value).toEqual({ start, end });
			expect(previous.next().serialize()).toBe(variant);
		}
	});
});
