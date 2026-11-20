import { expect, test } from "bun:test";

import { formatMetricVal, formatPercent } from "./metric-format";

test("formats missing, count, duration, and rate values", () => {
	expect(formatMetricVal(null, "views")).toBe("—");
	expect(formatMetricVal(0, "views")).toBe("0");
	expect(formatMetricVal(0.1, "views")).toBe("0.1");
	expect(formatMetricVal(1000, "views")).toBe("1k");
	expect(formatMetricVal(1000000, "views")).toBe("1M");
	expect(formatMetricVal(1000000000, "views")).toBe("1000M");
	expect(formatMetricVal(0.1, "avg_time_on_site")).toBe("00:00");
	expect(formatMetricVal(1, "avg_time_on_site")).toBe("00:01");
	expect(formatMetricVal(60, "avg_time_on_site")).toBe("01:00");
	expect(formatMetricVal(3600, "avg_time_on_site")).toBe("01:00:00");
	expect(formatMetricVal(0, "bounce_rate")).toBe("0.0%");
	expect(formatMetricVal(1, "bounce_rate")).toBe("100.0%");
	expect(formatMetricVal(0.999, "bounce_rate")).toBe("99.9%");
});

test("keeps a common unit across chart ticks", () => {
	expect(formatMetricVal(500, "views", 2000)).toBe("0.5k");
	expect(formatMetricVal(0, "views", 2000)).toBe("0");
});

test("formats percent changes", () => {
	expect(formatPercent(0)).toBe("0%");
	expect(formatPercent(1)).toBe("1%");
	expect(formatPercent(-1)).toBe("-1%");
	expect(formatPercent(0.1)).toBe("0.1%");
	expect(formatPercent(0.01)).toBe("0%");
	expect(formatPercent(1000)).toBe("1000%");
	expect(formatPercent(10000)).toBe("100x");
});
