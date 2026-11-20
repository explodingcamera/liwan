import { expect, test } from "bun:test";

import { DateRange } from "@/api/ranges";
import { formatDate, getTooltipDateRange } from "./format";

test("uses matching tooltip formats within the same year", () => {
	const current = new DateRange({ start: new Date(2026, 10, 5), end: new Date(2026, 10, 5, 23, 59, 59, 999) });
	const previous = new DateRange({ start: new Date(2026, 10, 4), end: new Date(2026, 10, 4, 23, 59, 59, 999) });
	const format = getTooltipDateRange(current.value.start, current.value.end, current, "hour", previous);
	expect(format).toBe("day+hour");
	expect(formatDate(current.value.start, format)).not.toContain("'26");
	expect(formatDate(previous.value.start, format)).not.toContain("'26");
});

test("shows years on both tooltip rows when periods span different years", () => {
	const current = new DateRange({ start: new Date(2026, 10, 1), end: new Date(2026, 10, 5) });
	const previous = new DateRange({ start: new Date(2025, 10, 1), end: new Date(2025, 10, 5) });
	const format = getTooltipDateRange(current.value.start, current.value.end, current, "day", previous);
	expect(formatDate(current.value.start, format)).toContain("'26");
	expect(formatDate(previous.value.start, format)).toContain("'25");
});
