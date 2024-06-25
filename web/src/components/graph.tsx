import { ResponsiveLine, type SliceTooltipProps } from "@nivo/line";
import { useMemo, useState } from "react";
import styles from "./graph.module.css";

export type DataPoint = {
	x: Date;
	y: number;
};

export const toDataPoints = (data: number[], range: { start: Date; end: Date }): DataPoint[] =>
	data.map((value, i) => ({
		x: new Date(range.start.getTime() + i * 1000 * 60 * 60 * 24),
		y: value,
	}));

export type DateRange = "year" | "month" | "day" | "hour";

const formatDate = (date: Date, range: DateRange = "day") => {
	switch (range) {
		case "year":
			return Intl.DateTimeFormat("en-US", { year: "numeric" }).format(date);
		case "month":
			return Intl.DateTimeFormat("en-US", { month: "short" }).format(date);
		case "day":
			return Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date);
		case "hour":
			return Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "numeric" }).format(date);
	}
};

// make sure parent container have a defined height when using
// responsive component, otherwise height will be 0 and
// no chart will be rendered.
// website examples showcase many properties,
// you'll often use just a few of them.
export const LineGraph = ({
	data,
	range = "day",
}: {
	data: DataPoint[];
	range?: DateRange;
}) => {
	const max = useMemo(() => Math.max(...data.map((d) => d.y)), [data]);
	const [currentSlice, setCurrentSlice] = useState(null);

	const Tooltip = (props: SliceTooltipProps) => {
		const point = props.slice.points[0].data;
		return (
			<div data-theme="dark" className={styles.tooltip}>
				<h2>Visitors</h2>
				<h3>
					<span>{formatDate(new Date(point.x), range)}</span> {point.y.toString()}
				</h3>
			</div>
		);
	};

	return (
		<ResponsiveLine
			data={[{ data, id: "data", color: "hsl(0, 70%, 50%)" }]}
			margin={{ top: 10, right: 1, bottom: 30, left: 40 }}
			xScale={{ type: "time" }}
			yScale={{
				type: "linear",
				min: 0,
				nice: true,
				max: Math.max(Math.ceil(max * 1.1), 10),
			}}
			enableGridX={false}
			enableArea={true}
			enablePoints={false}
			curve="monotoneX"
			yFormat=" >-.2f"
			axisTop={null}
			axisRight={null}
			axisBottom={{
				tickSize: 5,
				tickPadding: 5,
				tickRotation: 0,
				truncateTickAt: 0,
				format: (value: Date) => formatDate(value, range),
			}}
			axisLeft={{
				tickSize: 5,
				tickPadding: 5,
				tickRotation: 0,
				legend: "",
				legendOffset: -40,
				legendPosition: "middle",
				truncateTickAt: 0,
			}}
			pointSize={10}
			pointColor={{ theme: "background" }}
			pointBorderWidth={2}
			pointBorderColor={{ from: "serieColor" }}
			pointLabel="data.yFormatted"
			pointLabelYOffset={-12}
			enableSlices="x"
			sliceTooltip={Tooltip}
			enableTouchCrosshair={true}
			defs={[
				{
					colors: [
						{
							color: "inherit",
							offset: 0,
						},
						{
							color: "inherit",
							offset: 100,
							opacity: 0,
						},
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
