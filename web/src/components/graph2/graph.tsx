import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import styles from "./graph.module.css";

import { ResponsiveLine, type SliceTooltipProps } from "@nivo/line";
import { useWindowSize } from "@uidotdev/usehooks";
import { addMonths } from "date-fns";
import type { DataPoint } from ".";
import type { Metric } from "../../api";
import type { DateRange } from "../../api/ranges";
import { formatMetricVal } from "../../utils";
import { Tooltip } from "react-tooltip";
import { scaleLinear, scaleUtc } from "d3-scale";
import { extent } from "d3-array";
import { area, curveMonotoneX } from "d3-shape";
import { select } from "d3-selection";
import { easeCubic, easeLinear } from "d3-ease";

export type GraphRange = "year" | "month" | "day" | "hour";

const formatDate = (date: Date, range: GraphRange | "day+hour" | "day+year" = "day") => {
	switch (range) {
		case "day+year":
			return Intl.DateTimeFormat("en-US", { year: "numeric", month: "short", day: "numeric" }).format(date);
		case "year":
			return Intl.DateTimeFormat("en-US", { year: "numeric" }).format(date);
		case "month":
			return Intl.DateTimeFormat("en-US", { month: "short" }).format(addMonths(date, 1));
		case "day":
			return Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date);
		case "day+hour":
			return Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", hour: "numeric" }).format(date);
		case "hour":
			return Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "numeric" }).format(date);
	}
};

// const Tooltip = (props: SliceTooltipProps & { title: string; range: DateRange; metric: Metric }) => {
// 	const point = props.slice.points[0].data;
// 	const value = point.y.valueOf() as number;

// 	return (
// 		<div data-theme="dark" className={styles.tooltip}>
// 			<h2>{props.title}</h2>
// 			<h3>
// 				<span>{formatDate(new Date(point.x), props.range.getTooltipRange())}</span>{" "}
// 				{formatMetricVal(value, props.metric)}
// 			</h3>
// 		</div>
// 	);
// };

export const LineGraph2 = ({
	data,
	title,
	range,
	metric,
}: {
	data: DataPoint[];
	title: string;
	range: DateRange;
	metric: Metric;
}) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const svgBackgroundRef = useRef<SVGPathElement | null>(null);
	const svgLineRef = useRef<SVGPathElement | null>(null);
	const containerRef = useRef<HTMLDivElement | null>(null);

	// get the container size using a resize observer
	const [dimensions, setDimensions] = useState({ width: 800, height: 500 });
	useEffect(() => {
		if (containerRef.current) {
			const observer = new ResizeObserver((entries) => {
				for (const {
					contentRect: { width, height },
				} of entries) {
					setDimensions({ width, height });
				}
			});
			observer.observe(containerRef.current);
			return () => observer.disconnect();
		}
	}, []);

	const axisRange = range.getAxisRange();

	const updateGraph = useCallback(
		(transition: number, ease: (normalizedTime: number) => number = easeCubic) => {
			if (!svgBackgroundRef.current || !svgLineRef.current) return;

			const [minX, maxX] = extent(data, (d) => d.x);
			const [minY, maxY] = extent(data, (d) => d.y);

			const paddingBottom = 20;
			const xAxis = scaleUtc([minX || 0, maxX || 0], [0, dimensions.width]);
			const yAxis = scaleLinear([0, Math.max((maxY || 0) * 1.4, 20)], [dimensions.height - paddingBottom, 0]);

			const svgArea = area<DataPoint>()
				.x((d) => xAxis(d.x))
				.y0(yAxis(0) + paddingBottom)
				.y1((d) => yAxis(d.y));
			svgArea.length;

			const svgLine = area<DataPoint>()
				.x((d) => xAxis(d.x))
				.y((d) => yAxis(d.y));

			select(svgBackgroundRef.current)
				.transition()
				.ease(ease)
				.duration(transition)
				.attr("d", svgArea(data) || "");

			select(svgLineRef.current)
				.transition()
				.ease(ease)
				.duration(transition)
				.attr("d", svgLine(data) || "");
		},
		[dimensions, data],
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: only need to run this effect when dimensions change
	useEffect(() => updateGraph(20, easeLinear), [dimensions]);
	// biome-ignore lint/correctness/useExhaustiveDependencies: only need to run this effect when data changes
	useEffect(() => updateGraph(500), [data]);

	return (
		<div ref={containerRef} className={styles.graph} data-tooltip-float={true} data-tooltip-id="graph">
			<svg
				ref={svgRef}
				style={{ display: "block", width: "100%", height: "100%" }}
				// height={dimensions.height}
				// width={dimensions.width}
				// viewBox={`0 0 ${dimensions.width} ${dimensions.height}`}
			>
				<defs>
					<linearGradient id="graphGradient" x1="0" x2="0" y1="0" y2="1">
						<stop offset="0%" stopColor="rgba(166, 206, 227, 0.5)" />
						<stop offset="100%" stopColor="rgba(166, 206, 227, 0)" />
					</linearGradient>
				</defs>
				<title>Graph</title>
				<path ref={svgBackgroundRef} fill="url(#graphGradient)" stroke="none" />
				<path ref={svgLineRef} fill="none" stroke="#a6cee3" />
			</svg>

			<Tooltip id="map" className={styles.tooltipContainer} classNameArrow={styles.reset} disableStyleInjection>
				{/* <div data-theme="dark" className={styles.tooltip}>
                    <h2>{props.title}</h2>
                    <h3>
                        <span>{formatDate(new Date(point.x), props.range.getTooltipRange())}</span>{" "}
                        {formatMetricVal(value, props.metric)}
                    </h3>
                </div> */}
			</Tooltip>
		</div>
	);

	// return (
	// 	<ResponsiveLine
	// 		data={[{ data, id: "data", color: "hsl(0, 70%, 50%)" }]}
	// 		margin={{ top: 10, right: 40, bottom: 30, left: 40 }}
	// 		xScale={{ type: "time" }}
	// 		yScale={{
	// 			type: "linear",
	// 			nice: true,
	// 			min: 0,
	// 			max: Math.max(Math.ceil(max * 1.1), 5),
	// 		}}
	// 		enableGridX={false}
	// 		gridYValues={yCount}
	// 		enableArea={true}
	// 		enablePoints={false}
	// 		curve="monotoneX"
	// 		axisTop={null}
	// 		axisRight={null}
	// 		axisBottom={{
	// 			legend: "",
	// 			format: (value: Date) => formatDate(value, axisRange),
	// 			tickValues: xCount,
	// 		}}
	// 		axisLeft={{
	// 			legend: "",
	// 			tickValues: yCount,
	// 		}}
	// 		pointSize={10}
	// 		pointColor={{ theme: "background" }}
	// 		pointBorderWidth={2}
	// 		pointBorderColor={{ from: "serieColor" }}
	// 		pointLabel="data.yFormatted"
	// 		pointLabelYOffset={-12}
	// 		enableSlices="x"
	// 		sliceTooltip={tooltip}
	// 		defs={[
	// 			{
	// 				colors: [
	// 					{ color: "inherit", offset: 0 },
	// 					{ color: "inherit", offset: 100, opacity: 0 },
	// 				],
	// 				id: "gradientA",
	// 				type: "linearGradient",
	// 			},
	// 		]}
	// 		fill={[
	// 			{
	// 				id: "gradientA",
	// 				match: "*",
	// 			},
	// 		]}
	// 		colors={{
	// 			scheme: "paired",
	// 		}}
	// 		useMesh={true}
	// 		theme={{
	// 			crosshair: { line: { stroke: "var(--pico-color)", strokeWidth: 2 } },
	// 			axis: {
	// 				domain: {
	// 					line: { strokeWidth: 0 },
	// 				},
	// 				legend: {
	// 					text: { color: "var(--pico-color)" },
	// 				},

	// 				// axis label color
	// 				ticks: {
	// 					line: { strokeWidth: 0 },
	// 					text: { fill: "var(--pico-color)" },
	// 				},
	// 			},
	// 			grid: {
	// 				line: {
	// 					stroke: "var(--pico-secondary-background)",
	// 					strokeWidth: 0.3,
	// 				},
	// 			},
	// 		}}
	// 	/>
	// );
};
