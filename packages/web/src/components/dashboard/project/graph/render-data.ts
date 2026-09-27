import type { DateRange } from "@/api/ranges";
import type { DataPoint } from ".";

export const getGraphRenderData = (data: DataPoint[], range: DateRange, now = new Date()) => {
	const firstIncomplete =
		range.value.end > now ? data.findIndex((point) => range.getGraphBucketEnd(point.x) > now) : -1;

	return {
		domainMaxX: data[data.length - 1]?.x ?? now,
		solidLineData: firstIncomplete === -1 ? data : data.slice(0, firstIncomplete),
		dottedLineData: firstIncomplete === -1 ? [] : data.slice(Math.max(0, firstIncomplete - 1)),
	};
};
