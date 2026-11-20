import { describe, expect, test } from "bun:test";

import { DateRange } from "@/api/ranges";
import { alignComparisonData, getGraphRenderData, getSelectedGraphRange } from "./render-data";

const point = (hour: number) => ({ x: new Date(2026, 8, 27, hour), y: hour });

test("pairs comparison buckets from the start of each period", () => {
	const current = [point(10), point(11)];
	const previous = [point(1), point(2), point(3)];
	expect(alignComparisonData(current, previous)).toEqual([
		{ ...previous[0], x: current[0].x, date: previous[0].x },
		{ ...previous[1], x: current[1].x, date: previous[1].x },
	]);
	expect(alignComparisonData([], previous)).toEqual([]);
	expect(alignComparisonData(current)).toEqual([]);
});

test("leaves gaps after a shorter comparison", () => {
	const current = Array.from({ length: 31 }, (_, index) => ({ x: new Date(2025, 2, index + 1), y: index + 1 }));
	const previous = Array.from({ length: 28 }, (_, index) => ({ x: new Date(2025, 1, index + 1), y: index + 1 }));
	const aligned = alignComparisonData(current, previous);
	expect(aligned.map((point) => point.x)).toEqual(current.map((point) => point.x));
	expect(aligned[27]).toEqual({ x: current[27].x, y: 28, date: previous[27].x });
	expect(aligned.slice(28).map((point) => point.y)).toEqual([null, null, null]);
});

test("shows the rest of a full comparison after the current series ends", () => {
	const current = [point(10), point(11)];
	const previous = [point(1), point(2), point(3)];
	expect(alignComparisonData(current, previous, "hour").at(-1)).toEqual({
		x: point(12).x,
		y: 3,
		date: previous[2].x,
	});
});

describe("graph line styles", () => {
	test("dots the current and future buckets", () => {
		const now = new Date(2026, 8, 27, 12, 30);
		const data = [point(10), point(11), point(12), point(13), point(14)];
		const range = new DateRange({ start: point(0).x, end: new Date(2026, 8, 27, 23, 59, 59, 999) });
		const result = getGraphRenderData(data, range, range.getGraphInterval(), now);

		expect(result.solidLineData).toEqual(data.slice(0, 2));
		expect(result.dottedLineData).toEqual(data.slice(1));
	});

	test("keeps completed ranges solid", () => {
		const data = [point(10), point(11), point(12)];
		const range = new DateRange({ start: point(0).x, end: point(13).x });
		const result = getGraphRenderData(data, range, range.getGraphInterval(), new Date(2026, 8, 28));

		expect(result.solidLineData).toEqual(data);
		expect(result.dottedLineData).toEqual([]);
	});
});

describe("graph range selection", () => {
	test("snaps either drag direction to inclusive hourly buckets", () => {
		const data = [point(10), point(11), point(12), point(13), point(14)];
		const range = new DateRange({ start: point(10).x, end: new Date(2026, 8, 27, 14, 59, 59, 999) });
		const expected = { start: point(11).x, end: new Date(2026, 8, 27, 13, 59, 59, 999) };

		expect(getSelectedGraphRange(data, range, "hour", point(11).x, point(13).x)?.value).toEqual(expected);
		expect(getSelectedGraphRange(data, range, "hour", point(13).x, point(11).x)?.value).toEqual(expected);
		expect(getSelectedGraphRange(data, range, "hour", point(11).x, new Date(2026, 8, 27, 11, 10))?.value).toEqual({
			start: point(11).x,
			end: new Date(2026, 8, 27, 11, 59, 59, 999),
		});
		expect(getSelectedGraphRange(data, range, "hour", point(10).x, point(14).x)).toBeNull();
	});

	test("clips selected days to the original range", () => {
		const days = [1, 2, 3, 4].map((day) => ({ x: new Date(2026, 8, day), y: day }));
		const range = new DateRange({ start: new Date(2026, 8, 1, 10), end: new Date(2026, 8, 4, 12) });

		expect(getSelectedGraphRange(days, range, "day", days[0].x, days[2].x)?.value).toEqual({
			start: range.value.start,
			end: new Date(2026, 8, 3, 23, 59, 59, 999),
		});
	});

	test("selects whole calendar months", () => {
		const data = [0, 1, 2, 3].map((month) => ({ x: new Date(2024, month, 1), y: month }));
		const range = new DateRange({ start: data[0].x, end: new Date(2024, 3, 30, 23, 59, 59, 999) });

		expect(getSelectedGraphRange(data, range, "month", data[1].x, data[2].x)?.value).toEqual({
			start: new Date(2024, 1, 1),
			end: new Date(2024, 2, 31, 23, 59, 59, 999),
		});
	});
});
