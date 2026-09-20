import { useCallback, useEffect, useState } from "react";

import { DateRange } from "@/api/ranges";
import type { Metric } from "@/constants";

export type TimeFormat = "12h" | "24h";

export const getStoredTimeFormat = (): TimeFormat => {
	if (typeof window === "undefined") return "24h";
	const stored = window.localStorage?.getItem("liwan-time-format") || window.localStorage?.getItem("liwan/time-format");
	if (stored === "12h" || stored === "24h") return stored;
	return "24h";
};

export const setStoredTimeFormat = (format: TimeFormat) => {
	if (typeof window === "undefined") return;
	try {
		window.localStorage?.setItem("liwan-time-format", format);
		window.localStorage?.setItem("liwan/time-format", format);
	} catch {}
	window.dispatchEvent(new CustomEvent("liwan:time-format", { detail: format }));
};

export const useTimeFormat = () => {
	const [timeFormat, setTimeFormatState] = useState<TimeFormat>(getStoredTimeFormat);

	useEffect(() => {
		const handler = (e: Event) => {
			const customEvent = e as CustomEvent<TimeFormat>;
			if (customEvent.detail) {
				setTimeFormatState(customEvent.detail);
			} else {
				setTimeFormatState(getStoredTimeFormat());
			}
		};
		window.addEventListener("liwan:time-format", handler);
		window.addEventListener("storage", handler);
		return () => {
			window.removeEventListener("liwan:time-format", handler);
			window.removeEventListener("storage", handler);
		};
	}, []);

	const setTimeFormat = useCallback((format: TimeFormat) => {
		setTimeFormatState(format);
		setStoredTimeFormat(format);
	}, []);

	return { timeFormat, setTimeFormat };
};

export function formatEventTime(dateStr: string, timeFormat: TimeFormat = "24h"): string {
	try {
		const d = new Date(dateStr);
		return d.toLocaleTimeString([], {
			hour: timeFormat === "12h" ? "numeric" : "2-digit",
			minute: "2-digit",
			second: "2-digit",
			hour12: timeFormat === "12h",
		});
	} catch {
		return dateStr;
	}
}

export const useMetric = () => {
	const [metric, _setMetric] = useState<Metric>(
		() => (localStorage.getItem("liwan/selected-metric") ?? "views") as Metric,
	);
	const setMetric = useCallback((metric: Metric) => {
		_setMetric(metric);
		localStorage.setItem("liwan/selected-metric", metric);
	}, []);
	return { metric, setMetric };
};

export const useRange = () => {
	const [range, _setRange] = useState<DateRange>(() =>
		DateRange.deserialize(localStorage.getItem("liwan/date-range") || "last30Days"),
	);
	const setRange = useCallback((range: DateRange) => {
		_setRange(range);
		localStorage.setItem("liwan/date-range", range.serialize());
	}, []);
	return { range, setRange };
};
