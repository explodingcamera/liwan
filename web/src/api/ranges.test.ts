import { describe, expect, it } from "bun:test";
import { differenceInCalendarDays, endOfDay, startOfDay, startOfMonth, subDays, subMonths } from "date-fns";

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

	it("should calculate if the range ends today", () => {
		const now = startOfDay(new Date());
		const range = new DateRange({ start: now, end: now });
		expect(range.endsToday()).toBe(true);

		const yesterday = subDays(now, 1);
		const pastRange = new DateRange({ start: yesterday, end: yesterday });
		expect(pastRange.endsToday()).toBe(false);
	});

	it("should calculate graph range and data points", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 7));
		const range = new DateRange({ start, end });
		expect(range.getGraphRange()).toBe("day");
		expect(range.getGraphInterval()).toBe("day");
	});

	it("should use inclusive calendar hours for short multi-day ranges", () => {
		const start = startOfDay(new Date(2024, 10, 1));
		const end = endOfDay(new Date(2024, 10, 2));
		const range = new DateRange({ start, end });

		expect(range.getGraphRange()).toBe("hour");
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

	it("should keep last7DaysHourly at seven calendar days", () => {
		const { start, end } = ranges.last7DaysHourly().range;

		expect(differenceInCalendarDays(end, start) + 1).toBe(7);
		expect(new DateRange("last7DaysHourly").getGraphInterval()).toBe("hour");
	});

	it("should keep last12Months to the current month and never future months", () => {
		const now = new Date();
		const { start, end } = ranges.last12Months().range;

		expect(start).toEqual(startOfMonth(subMonths(now, 11)));
		expect(end).toEqual(endOfDay(now));
	});

	it("should include years on axis and tooltip labels for ranges spanning calendar years", () => {
		const range = new DateRange({ start: new Date(2024, 11, 30), end: new Date(2025, 0, 2) });

		expect(range.getAxisRange()).toBe("day+year");
		expect(range.getTooltipRange()).toBe("day+hour");
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
		const weekShort = new DateRange({ start: startOfDay(new Date(2024, 10, 1)), end: endOfDay(new Date(2024, 10, 3)) });
		weekShort.variant = "weekToDate";
		expect(weekShort.getGraphInterval()).toBe("hour");

		const weekLong = new DateRange({ start: startOfDay(new Date(2024, 10, 1)), end: endOfDay(new Date(2024, 10, 4)) });
		weekLong.variant = "weekToDate";
		expect(weekLong.getGraphInterval()).toBe("day");

		const monthShort = new DateRange({
			start: startOfDay(new Date(2024, 10, 1)),
			end: endOfDay(new Date(2024, 10, 6)),
		});
		monthShort.variant = "monthToDate";
		expect(monthShort.getGraphInterval()).toBe("hour");

		const monthLong = new DateRange({ start: startOfDay(new Date(2024, 10, 1)), end: endOfDay(new Date(2024, 10, 7)) });
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
});
