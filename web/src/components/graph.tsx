import { ResponsiveLine } from "@nivo/line";
import { useMemo } from "react";

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

	return (
		<ResponsiveLine
			data={[{ data, id: "data" }]}
			margin={{ top: 10, right: 30, bottom: 30, left: 50 }}
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
			sliceTooltip={(props) => {
				const point = props.slice.points[0].data;
				return (
					<div
						style={{
							backgroundColor: "rgba(0, 0, 0, 0.4)",
							padding: ".4rem .5rem .6rem .5rem",
							borderRadius: ".4rem",
						}}
					>
						<h6 style={{ marginBottom: ".3rem" }}>Visitors</h6>
						<div>
							<span
								style={{
									color: "black",
									backgroundColor: "rgba(255, 255, 255, 0.8)",
									padding: ".1rem .2rem",
									borderRadius: ".2rem",
								}}
							>
								{formatDate(new Date(point.x), range)}:
							</span>{" "}
							{point.y.toString()}
						</div>
					</div>
				);
			}}
			enableTouchCrosshair={true}
			useMesh={true}
			theme={{
				axis: {
					domain: {
						line: {
							stroke: "#777777",
							strokeWidth: 1,
						},
					},
				},
			}}
		/>
	);
};
