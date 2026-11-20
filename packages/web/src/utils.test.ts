import { describe, expect, test } from "bun:test";

import { capitalizeAll, cls, countryCodeToFlag, formatDateRange, fuzzyFilter } from "./utils";

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

	test("formatDateRange", () => {
		const options = { today: new Date(2023, 10, 15, 12), locale: "en-US" };
		const format = (from: Date, to: Date) => formatDateRange(from, to, options);

		expect(format(new Date(2023, 0, 1), new Date(2023, 0, 12, 23, 59, 59, 999))).toBe("Jan 1 - 12");
		expect(format(new Date(2023, 0, 3), new Date(2023, 3, 20, 23, 59, 59, 999))).toBe("Jan 3 - Apr 20");
		expect(format(new Date(2022, 0, 1), new Date(2023, 0, 20, 23, 59, 59, 999))).toBe("Jan 1 '22 - Jan 20 '23");
		expect(format(new Date(2023, 0, 1), new Date(2023, 0, 1, 23, 59, 59, 999))).toBe("Sun, Jan 1");
		expect(format(new Date(2022, 0, 1), new Date(2022, 0, 1, 23, 59, 59, 999))).toBe("Sat, Jan 1, 2022");
		expect(format(new Date(2023, 0, 1, 0, 11), new Date(2023, 0, 1, 14, 30, 59, 999))).toBe("Jan 1, 12:11am - 2:30pm");
		expect(format(new Date(2023, 0, 1, 0, 11), new Date(2023, 0, 2, 14, 30))).toBe("Jan 1, 12:11am - Jan 2, 2:30pm");
		expect(format(new Date(2023, 10, 15, 12), new Date(2023, 10, 15, 13))).toBe("12pm - 1pm");
		expect(format(new Date(2023, 3, 1), new Date(2023, 3, 30, 23, 59, 59, 999))).toBe("April 2023");
		expect(format(new Date(2023, 0, 1), new Date(2023, 1, 28, 23, 59, 59, 999))).toBe("Jan - Feb 2023");
		expect(format(new Date(2023, 0, 1), new Date(2023, 2, 31, 23, 59, 59, 999))).toBe("Q1 2023");
		expect(format(new Date(2023, 0, 1), new Date(2023, 11, 31, 23, 59, 59, 999))).toBe("2023");
	});

	test("fuzzyFilter", () => {
		const items = [
			{ name: "signup_completed", label: null },
			{ name: "page_view", label: "Page View" },
			{ name: "button_click", label: 42 },
			{ name: "sign_in", label: "Sign In" },
		];
		const search = (query: string) => fuzzyFilter(query, items, ["name", "label"]).map((item) => item.name);

		expect(search("sign")).toEqual(["signup_completed", "sign_in"]);
		expect(search("VIEW")).toEqual(["page_view"]);
		expect(search("sgnin")).toEqual(["sign_in"]);
		expect(search("bc")).toEqual(["button_click"]);
		expect(search("42")).toEqual(["button_click"]);
		expect(search("xyz")).toEqual([]);
		expect(search("view page")).toEqual(["page_view"]);
		expect(search("sign completed")).toEqual(["signup_completed"]);
		expect(search("sign xyz")).toEqual([]);
		expect(search("  ")).toEqual(items.map((item) => item.name));
	});
});
