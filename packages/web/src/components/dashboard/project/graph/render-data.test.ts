import { describe, expect, test } from "bun:test";

import { DateRange } from "@/api/ranges";
import { getGraphRenderData, getSelectedGraphRange } from "./render-data";

const point = (hour: number) => ({ x: new Date(2026, 8, 27, hour), y: hour });

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
