import { describe, expect, test } from "bun:test";

import { DateRange } from "@/api/ranges";
import { getGraphRenderData } from "./render-data";

const point = (hour: number) => ({ x: new Date(2026, 8, 27, hour), y: hour });

describe("graph line styles", () => {
	test("dots the current and future buckets", () => {
		const now = new Date(2026, 8, 27, 12, 30);
		const data = [point(10), point(11), point(12), point(13), point(14)];
		const range = new DateRange({ start: point(0).x, end: new Date(2026, 8, 27, 23, 59, 59, 999) });
		const result = getGraphRenderData(data, range, now);

		expect(result.solidLineData).toEqual(data.slice(0, 2));
		expect(result.dottedLineData).toEqual(data.slice(1));
	});

	test("keeps completed ranges solid", () => {
		const data = [point(10), point(11), point(12)];
		const range = new DateRange({ start: point(0).x, end: point(13).x });
		const result = getGraphRenderData(data, range, new Date(2026, 8, 28));

		expect(result.solidLineData).toEqual(data);
		expect(result.dottedLineData).toEqual([]);
	});
});
