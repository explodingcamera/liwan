import { differenceInHours, isSameYear } from "date-fns";

import type { DateRange } from "@/api/ranges";
import type { GraphInterval } from "@/constants";

type DateDisplayRange = "day" | "hour" | "day+hour" | "day+hour+year" | "day+year";

const keepMeridiemTogether = (value: string) => value.replace(/(\d)\s([AP]M)\b/g, "$1\u00A0$2");
const formatShortYear = (date: Date) => `'${String(date.getFullYear()).slice(-2)}`;

/** Format a date consistently for graph labels and tooltips. */
export const formatDate = (date: Date, range: DateDisplayRange = "day") => {
	switch (range) {
		case "day+year":
			return `${Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date)} ${formatShortYear(date)}`;
		case "day":
			return Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date);
		case "day+hour":
			return keepMeridiemTogether(
				Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", hour: "numeric" }).format(date),
			);
		case "day+hour+year":
			return keepMeridiemTogether(
				`${Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", hour: "numeric" }).format(date)} ${formatShortYear(date)}`,
			);
		case "hour":
			return keepMeridiemTogether(Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "numeric" }).format(date));
	}
};

/** Choose the date detail required by the current axis. */
export const getAxisDateRange = (start: Date, end: Date): "hour" | "day" | "day+year" => {
	if (differenceInHours(end, start) <= 24) return "hour";
	if (!isSameYear(start, end)) return "day+year";
	return "day";
};

/** Use one tooltip format for both periods, including years only when they differ. */
export const getTooltipDateRange = (
	start: Date,
	end: Date,
	range: DateRange,
	interval: GraphInterval,
	comparisonRange?: DateRange,
): DateDisplayRange => {
	const showYear =
		!isSameYear(start, end) ||
		(comparisonRange !== undefined &&
			(!isSameYear(start, comparisonRange.value.start) || !isSameYear(end, comparisonRange.value.end)));
	if (interval !== "hour") return showYear ? "day+year" : "day";
	if (!comparisonRange && range.value.start.toDateString() === range.value.end.toDateString()) return "hour";
	return showYear ? "day+hour+year" : "day+hour";
};
