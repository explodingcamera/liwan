import { describe, expect, test } from "bun:test";
import { capitalizeAll, cls, countryCodeToFlag, formatMetricVal, formatPercent } from "./utils";

describe("utils", () => {
	test("capitalizeAll", () => {
		expect(capitalizeAll("hello world")).toBe("Hello World");
		expect(capitalizeAll("hello-world")).toBe("Hello-world");
		expect(capitalizeAll("hello_world")).toBe("Hello_world");
		expect(capitalizeAll("helloWorld")).toBe("HelloWorld");
		expect(capitalizeAll("HELLO WORLD")).toBe("HELLO WORLD");
	});

	test("cls", () => {
		expect(cls("a", "b", "c", null)).toBe("a b c");
		expect(cls(["a", "b", undefined, null, "c"])).toBe("a b c");
		expect(cls(undefined, [null], ["a", "b", undefined, null, "c"])).toBe("a b c");
	});

	test("countryCodeToFlag", () => {
		expect(countryCodeToFlag("us")).toBe("🇺🇸");
		expect(countryCodeToFlag("gb")).toBe("🇬🇧");
		expect(countryCodeToFlag("de")).toBe("🇩🇪");
		expect(countryCodeToFlag("fr")).toBe("🇫🇷");
		expect(countryCodeToFlag("es")).toBe("🇪🇸");
		expect(countryCodeToFlag("")).toBe("🇽🇽");
	});

	test("formatMetricVal", () => {
		expect(formatMetricVal(0.1, "views")).toBe("0.1");
		expect(formatMetricVal(0, "views")).toBe("0");
		expect(formatMetricVal(1, "views")).toBe("1");
		expect(formatMetricVal(1000, "views")).toBe("1k");
		expect(formatMetricVal(1000000, "views")).toBe("1M");
		expect(formatMetricVal(1000000000, "views")).toBe("1000M");
		expect(formatMetricVal(0.1, "avg_time_on_site")).toBe("00:00");
		expect(formatMetricVal(1, "avg_time_on_site")).toBe("00:01");
		expect(formatMetricVal(60, "avg_time_on_site")).toBe("01:00");
		expect(formatMetricVal(3600, "avg_time_on_site")).toBe("01:00:00");
		expect(formatMetricVal(1, "bounce_rate")).toBe("100%");
		expect(formatMetricVal(0.92, "bounce_rate")).toBe("92%");
		expect(formatMetricVal(0.999, "bounce_rate")).toBe("99.9%");
	});

	test("formatPercent", () => {
		expect(formatPercent(0)).toBe("0%");
		expect(formatPercent(1)).toBe("1%");
		expect(formatPercent(0.1)).toBe("0.1%");
		expect(formatPercent(0.01)).toBe("0%");
		expect(formatPercent(0.001)).toBe("0%");
		expect(formatPercent(1000)).toBe("1000%");
		expect(formatPercent(10000)).toBe("100x");
	});
});
