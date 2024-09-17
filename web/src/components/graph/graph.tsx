import { useMemo } from "react";
import styles from "./graph.module.css";

import { ResponsiveLine, type SliceTooltipProps } from "@nivo/line";
import { addMonths } from "date-fns";
import type { DataPoint } from ".";
import { formatMetricVal } from "../../utils";

export type GraphRange = "year" | "month" | "day" | "hour";

const formatDate = (date: Date, range: GraphRange = "day") => {
	switch (range) {
		case "year":
			return Intl.DateTimeFormat("en-US", { year: "numeric" }).format(date);
		case "month":
			return Intl.DateTimeFormat("en-US", { month: "short" }).format(addMonths(date, 1));
		case "day":
			return Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date);
		case "hour":
			return Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "numeric" }).format(date);
	}
};

const Tooltip = (props: SliceTooltipProps & { title: string; range: GraphRange }) => {
	const point = props.slice.points[0].data;
	const value = point.y.valueOf() as number;

	return (
		<div data-theme="dark" className={styles.tooltip}>
			<h2>{props.title}</h2>
			<h3>
				<span>{formatDate(new Date(point.x), props.range)}</span> {formatMetricVal(value)}
			</h3>
		</div>
	);
};

export const LineGraph = ({
	data,
	title,
	range = "day",
}: {
	data: DataPoint[];
	title: string;
	range?: GraphRange;
}) => {
	const max = useMemo(() => Math.max(...data.map((d) => d.y)), [data]);
	const yCount = 5;

	return (
		<ResponsiveLine
			data={[{ data, id: "data", color: "hsl(0, 70%, 50%)" }]}
			margin={{ top: 10, right: 40, bottom: 30, left: 40 }}
			xScale={data.length > 14 ? { type: "time" } : { type: "point" }}
			yScale={{
				type: "linear",
				nice: true,
				min: 0,
				max: Math.max(Math.ceil(max * 1.1), 5),
			}}
			enableGridX={false}
			gridYValues={yCount}
			enableArea={true}
			enablePoints={false}
			curve="monotoneX"
			axisTop={null}
			axisRight={null}
			axisBottom={{
				legend: "",
				format: (value: Date) => formatDate(value, range),
			}}
			axisLeft={{
				legend: "",
				tickValues: yCount,
			}}
			pointSize={10}
			pointColor={{ theme: "background" }}
			pointBorderWidth={2}
			pointBorderColor={{ from: "serieColor" }}
			pointLabel="data.yFormatted"
			pointLabelYOffset={-12}
			enableSlices="x"
			sliceTooltip={(props) => <Tooltip {...props} title={title} range={range} />}
			defs={[
				{
					colors: [
						{ color: "inherit", offset: 0 },
						{ color: "inherit", offset: 100, opacity: 0 },
					],
					id: "gradientA",
					type: "linearGradient",
				},
			]}
			fill={[
				{
					id: "gradientA",
					match: "*",
				},
			]}
			colors={{
				scheme: "paired",
			}}
			useMesh={true}
			theme={{
				crosshair: { line: { stroke: "var(--pico-color)", strokeWidth: 2 } },
				axis: {
					domain: {
						line: { strokeWidth: 0 },
					},
					legend: {
						text: { color: "var(--pico-color)" },
					},

					// axis label color
					ticks: {
						line: { strokeWidth: 0 },
						text: { fill: "var(--pico-color)" },
					},
				},
				grid: {
					line: {
						stroke: "var(--pico-secondary-background)",
						strokeWidth: 0.3,
					},
				},
			}}
		/>
	);
};
