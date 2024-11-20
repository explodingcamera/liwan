import { capitalizeAll, cls, countryCodeToFlag, formatMetricVal, formatPercent } from "./utils";
import { expect, test, describe } from "bun:test";

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
	});

	test("formatMetricVal", () => {
		expect(formatMetricVal(0)).toBe("0");
		expect(formatMetricVal(1)).toBe("1");
		expect(formatMetricVal(1000)).toBe("1k");
		expect(formatMetricVal(1000000)).toBe("1M");
		expect(formatMetricVal(1000000000)).toBe("1000M");
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
