import { useCallback, useState } from "react";
import type { Metric } from "../api";
import { DateRange } from "../api/ranges";

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
