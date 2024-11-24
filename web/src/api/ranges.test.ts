import { describe, expect, it } from "bun:test";
import { addDays, endOfDay, startOfDay, subDays } from "date-fns";
import { DateRange } from "./ranges";

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
		const start = new Date(2024, 10, 1);
		const end = new Date(2024, 10, 15);
		const range = new DateRange({ start, end });
		expect(range.getGraphRange()).toBe("day");
		expect(range.getGraphDataPoints()).toBe(14);
	});
});
