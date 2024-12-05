import { useCallback, useEffect, useRef, useState } from "react";
import { Tooltip } from "react-tooltip";
import styles from "./graph.module.css";

import { extent } from "d3-array";
import { easeCubic, easeCubicOut } from "d3-ease";
import { scaleLinear, scaleTime } from "d3-scale";
import { select } from "d3-selection";
import { area } from "d3-shape";
import "d3-transition";

import { addMonths } from "date-fns";

import type { DataPoint } from ".";
import type { Metric } from "../../api";
import type { DateRange } from "../../api/ranges";
import { debounce, formatMetricVal, formatMetricValEvenly } from "../../utils";
import { axisBottom, axisLeft } from "./axis";

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

export const LineGraph = ({
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
	const containerRef = useRef<HTMLDivElement | null>(null);

	// get the container size using a resize observer
	const [dimensions, setDimensions] = useState<{ width: number; height: number } | null>(null);
	useEffect(() => {
		if (containerRef.current) {
			const observer = new ResizeObserver((entries) => {
				for (const {
					contentRect: { width, height },
				} of entries) {
					// setDimensions({ width, height });
					debounce(() => setDimensions({ width, height }), 100)();
				}
			});
			observer.observe(containerRef.current);
			return () => observer.disconnect();
		}
	}, []);

	const axisRange = range.getAxisRange();

	const updateGraph = useCallback(
		(transition: number, ease: (normalizedTime: number) => number = easeCubic) => {
			if (!svgRef.current || !dimensions) return;
			const svg = select(svgRef.current);

			const [minX, maxX] = extent(data, (d) => d.x).map((d) => d || new Date());
			const [_minY, maxY] = extent(data, (d) => d.y).map((d) => d || 0);

			const sizeX = maxX.getTime() - minX.getTime();

			let xCount = Math.min(data.length, 8);
			if (dimensions.width && dimensions.width < 500) {
				xCount = Math.min(data.length, 6);
			} else if (dimensions.width && dimensions.width < 400) {
				xCount = Math.min(data.length, 4);
			}

			const paddingTop = 30;
			const paddingBottom = 40;

			const xAxis = scaleTime().domain([minX, maxX]).range([0, dimensions.width]);

			const yAxis =
				metric === "bounce_rate"
					? scaleLinear([0, 1], [dimensions.height - paddingBottom - paddingTop, 0])
					: scaleLinear([0, Math.max(maxY * 1.25 || 0, 20)], [dimensions.height - paddingBottom - paddingTop, 0]);

			const svgArea = area<DataPoint>()
				.x((d) => xAxis(d.x))
				.y0(yAxis(0))
				.y1((d) => yAxis(d.y));

			const svgLine = area<DataPoint>()
				.x((d) => xAxis(d.x))
				.y((d) => yAxis(d.y));

			svg
				.selectChild("#background")
				.attr("transform", `translate(0, ${paddingTop})`)
				.transition()
				.ease(easeCubic)
				.duration(transition)
				.attr("d", svgArea(data) || "");

			svg
				.selectChild("#line")
				.attr("transform", `translate(0, ${paddingTop})`)
				.transition()
				.ease(easeCubic)
				.duration(transition)
				.attr("d", svgLine(data) || "");

			const yAxisElement = svg.selectChild<SVGGElement>("#y-axis");
			const xAxisElement = svg.selectChild<SVGGElement>("#x-axis");

			let tickValuesY = yAxis.ticks(5).filter((d) => d !== 0);
			// if more than 5 ticks, remove every other tick
			if (tickValuesY.length > 5) {
				tickValuesY = tickValuesY.filter((_, i) => i % 2 === 0);
			}

			const leftAxis = axisLeft(yAxis)
				.disableDomain()
				.tickFormat((d) => formatMetricValEvenly(d as number, metric, maxY))
				.tickValues(tickValuesY);

			let tickValuesX = xAxis.ticks(xCount);
			if (xAxis(tickValuesX[0]) < 20) tickValuesX = tickValuesX.slice(1);
			if (xAxis(tickValuesX[tickValuesX.length - 1]) > dimensions.width - 20) tickValuesX = tickValuesX.slice(0, -1);

			const bottomAxis = axisBottom(xAxis)
				.disableDomain()
				.disableTicks()
				.tickFormat((d) => formatDate(d as Date, axisRange))
				.tickValues(tickValuesX);

			xAxisElement
				.transition()
				.ease(easeCubic)
				.duration(transition)
				.call((ax) => {
					bottomAxis(ax);
					ax.attr("transform", `translate(0, ${dimensions.height - paddingBottom + 10})`);
				});

			yAxisElement.call((ax) => {
				leftAxis(ax);
				ax.attr("transform", `translate(0, ${paddingTop})`);

				// make the ticks take up the full width of the graph
				ax.selectAll(".tick line")
					.attr("x2", dimensions.width)
					.attr("stroke", "var(--pico-secondary-background)")
					.attr("stroke-width", 0.5)
					.attr("stroke-dasharray", "2, 2");

				// move the labels to the right side of the graph, keeping the text right-aligned
				ax.selectAll(".tick text")
					.attr("x", dimensions.width - 35)
					.attr("text-anchor", "end")
					.attr("dx", "2em")
					.attr("dy", "0.3em");

				// remove existing blocks
				ax.selectAll("rect").remove();

				// add black blocks before the lables with the same width as the text
				ax.selectAll(".tick text").each(function () {
					const text = select(this as SVGTextElement);
					const textWidth = text.node()?.getBBox().width || 0;
					// biome-ignore lint/suspicious/noExplicitAny: can't infer the type of the parent node
					select((this as any)?.parentNode as SVGGElement)
						.insert("rect", ":nth-child(2)")
						.attr("x", dimensions.width - textWidth - 19)
						.attr("y", -11)
						.attr("width", textWidth + 10)
						.attr("opacity", 0.6)
						.attr("height", 20)
						.attr("rx", 6)
						.attr("fill", "var(--pico-card-background-color)");
				});
			});
		},
		[dimensions, data, axisRange, metric],
	);

	useEffect(() => {
		if (!svgRef.current || !dimensions) return;

		const mouseMove = (event: MouseEvent) => {
			if (!svgRef.current) return;
			const tooltip = select(svgRef.current).selectChild("#tooltip");
			const tooltipRect = (tooltip.node() as SVGForeignObjectElement | null)?.getBoundingClientRect();
			const tooltipWidth = tooltipRect?.width || 0;
			const tooltipHeight = tooltipRect?.height || 0;
			const tooltipPadding = 10;

			const svgRect = svgRef.current.getBoundingClientRect();
			const svgWidth = svgRect.width;
			const svgHeight = svgRect.height;

			// Determine if cursor is on the left or right side of the SVG
			const isLeftSide = event.clientX - svgRect.left < svgWidth / 2;

			// Calculate tooltip X position based on cursor side
			const tooltipX = isLeftSide
				? Math.min(event.clientX - svgRect.left + tooltipPadding, svgWidth - tooltipWidth - tooltipPadding)
				: Math.max(event.clientX - svgRect.left - tooltipWidth - tooltipPadding, tooltipPadding);

			// Calculate tooltip Y position
			const tooltipY = Math.min(
				event.clientY - svgRect.top + tooltipPadding - tooltipHeight / 3,
				svgHeight - tooltipHeight - tooltipPadding,
			);

			tooltip.transition().duration(200).ease(easeCubicOut).attr("x", tooltipX).attr("y", tooltipY).attr("opacity", 1);

			const needle = select(svgRef.current).selectChild("#needle");
			const x = event.clientX - svgRect.left - 1;
			const step = (dimensions.width - 2) / (data.length - 1);
			const index = Math.min(Math.max(Math.round(x / step), 0), data.length - 1); // Clamp index

			const snappedX = 1 + index * step; // Snap to the clamped index
			needle
				.transition()
				.duration(200)
				.ease(easeCubicOut)
				.attr("d", `M ${snappedX} 0 L ${snappedX} ${svgHeight - 40}`);

			const point = data[index];
			const value = point.y;

			const date = new Date(point.x);
			const dateRange = range.getTooltipRange();

			const tooltipDate = formatDate(date, dateRange);
			const tooltipValue = formatMetricVal(value, metric);

			tooltip.select(".date").text(tooltipDate);
			tooltip.select(".value").text(tooltipValue);
		};

		const mouseLeave = () => {
			select(svgRef.current).selectChild("#tooltip").interrupt().attr("opacity", 0);
			select(svgRef.current).selectChild("#needle").interrupt().attr("d", "M 0 0 L 0 0");
		};

		svgRef.current.addEventListener("mousemove", mouseMove);
		svgRef.current.addEventListener("mouseleave", mouseLeave);

		return () => {
			svgRef.current?.removeEventListener("mousemove", mouseMove);
			svgRef.current?.removeEventListener("mouseleave", mouseLeave);
		};
	});

	const firstRender = useRef(true);

	useEffect(() => {
		if (firstRender.current && dimensions && data) {
			firstRender.current = false;
			updateGraph(0);
		}
	}, [dimensions, updateGraph, data]);

	useEffect(() => {
		if (!firstRender.current) updateGraph(300);
	}, [updateGraph]);

	return (
		<div ref={containerRef} className={styles.graph} data-tooltip-float={true} data-tooltip-id="graph">
			<svg ref={svgRef} style={{ display: "block", width: "100%", height: "100%" }}>
				<defs>
					<linearGradient id="graphGradient" x1="0" x2="0" y1="0" y2="1">
						<stop offset="0%" stopColor="rgba(166, 206, 227, 0.4)" />
						<stop offset="100%" stopColor="rgba(166, 206, 227, 0)" />
					</linearGradient>
				</defs>
				<title>Graph</title>
				<path id="background" fill="url(#graphGradient)" stroke="none" />
				<path id="line" fill="none" stroke="#a6cee3" />
				{/* doted line */}
				<path
					id="needle"
					fill="none"
					stroke="var(--pico-secondary-background)"
					strokeDasharray="5, 5"
					strokeWidth="2"
				/>
				<foreignObject id="tooltip" width="170" height="100" opacity="0">
					<div data-theme="dark" className={styles.tooltip}>
						<h2>{title}</h2>
						<h3>
							<span className="date" /> <span className="value" />
							{/* <span>{formatDate(new Date(data[data.length - 1].x), range.getTooltipRange())}</span>{" "}
							{formatMetricVal(0, metric)} */}
						</h3>
					</div>
				</foreignObject>
				<g id="y-axis" />
				<g id="x-axis" />
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
};
