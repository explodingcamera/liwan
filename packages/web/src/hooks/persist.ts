import { useCallback, useEffect, useState } from "react";

import { DateRange } from "@/api/ranges";
import type { GraphInterval, Metric } from "@/constants";

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
	const [{ range, interval }, setSelection] = useState<{ range: DateRange; interval: GraphInterval | "auto" }>(() => {
		try {
			const saved = JSON.parse(localStorage.getItem("liwan/date-range") || "{}") as {
				range?: string;
				interval?: GraphInterval | "auto";
			};
			return { range: DateRange.deserialize(saved.range ?? "last30Days"), interval: saved.interval ?? "auto" };
		} catch {
			return { range: new DateRange("last30Days"), interval: "auto" };
		}
	});
	useEffect(() => {
		localStorage.setItem("liwan/date-range", JSON.stringify({ range: range.serialize(), interval }));
	}, [range, interval]);
	const setRange = useCallback((range: DateRange) => {
		setSelection((current) => ({ ...current, range, interval: "auto" }));
	}, []);
	const setInterval = useCallback((value: GraphInterval | "auto") => {
		setSelection((current) => ({ ...current, interval: value }));
	}, []);
	return { range, setRange, interval, setInterval };
};
