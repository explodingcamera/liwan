import { DateRange } from "@/api/ranges";
import type { GraphInterval } from "@/constants";
import type { DataPoint } from ".";

export const getGraphRenderData = (data: DataPoint[], range: DateRange, interval: GraphInterval, now = new Date()) => {
	const firstIncomplete =
		range.value.end > now ? data.findIndex((point) => range.getGraphBucketEnd(point.x, interval) > now) : -1;

	return {
		domainMaxX: data[data.length - 1]?.x ?? now,
		solidLineData: firstIncomplete === -1 ? data : data.slice(0, firstIncomplete),
		dottedLineData: firstIncomplete === -1 ? [] : data.slice(Math.max(0, firstIncomplete - 1)),
	};
};

export const getGraphSelectionIndices = (data: DataPoint[], start: Date, end: Date): [number, number] | null => {
	if (data.length < 2) return null;
	const nearestIndex = (date: Date) =>
		data.reduce(
			(nearest, point, index) =>
				Math.abs(point.x.getTime() - date.getTime()) < Math.abs(data[nearest].x.getTime() - date.getTime())
					? index
					: nearest,
			0,
		);
	const first = nearestIndex(start);
	const last = nearestIndex(end);
	return [Math.min(first, last), Math.max(first, last)];
};

export const getSelectedGraphRange = (
	data: DataPoint[],
	range: DateRange,
	interval: GraphInterval,
	start: Date,
	end: Date,
): DateRange | null => {
	const indices = getGraphSelectionIndices(data, start, end);
	if (!indices) return null;
	const [first, last] = indices;
	const selectedStart = new Date(Math.max(data[first].x.getTime(), range.value.start.getTime()));
	const selectedEnd = new Date(
		Math.min(range.getGraphBucketEnd(data[last].x, interval).getTime() - 1, range.value.end.getTime()),
	);
	if (selectedStart >= selectedEnd || (selectedStart <= range.value.start && selectedEnd >= range.value.end))
		return null;
	return new DateRange({ start: selectedStart, end: selectedEnd });
};
