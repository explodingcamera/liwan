import styles from "./linegraph.module.css";

import { useEffect, useRef, useState } from "react";
import { extent } from "d3-array";
import { easeCubic } from "d3-ease";
import { scaleLinear, scaleTime } from "d3-scale";
import { select } from "d3-selection";
import { area, line } from "d3-shape";
import "d3-transition";

import { addMonths, differenceInHours, isSameYear } from "date-fns";

import type { DateRange } from "@/api/ranges";
import type { Metric } from "@/constants";
import { formatMetricVal, formatMetricValEvenly } from "@/utils";
import type { DataPoint } from ".";
import { axisBottom, axisLeft } from "./axis";

export type GraphRange = "year" | "month" | "day" | "hour";
type DateDisplayRange = GraphRange | "day+hour" | "day+hour+year" | "day+year";

const keepMeridiemTogether = (value: string) => value.replace(/(\d)\s([AP]M)\b/g, "$1\u00A0$2");
const formatShortYear = (date: Date) => `'${String(date.getFullYear()).slice(-2)}`;
const formatDate = (date: Date, range: DateDisplayRange = "day") => {
	switch (range) {
		case "day+year":
			return `${Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" }).format(date)} ${formatShortYear(date)}`;
		case "year":
			return Intl.DateTimeFormat("en-US", { year: "numeric" }).format(date);
		case "month":
			return Intl.DateTimeFormat("en-US", { month: "short" }).format(addMonths(date, 1));
		case "day":
			return Intl.DateTimeFormat("en-US", {
				month: "short",
				day: "numeric",
			}).format(date);
		case "day+hour":
			return keepMeridiemTogether(
				Intl.DateTimeFormat("en-US", {
					month: "short",
					day: "numeric",
					hour: "numeric",
				}).format(date),
			);
		case "day+hour+year":
			return keepMeridiemTogether(
				`${Intl.DateTimeFormat("en-US", {
					month: "short",
					day: "numeric",
					hour: "numeric",
				}).format(date)} ${formatShortYear(date)}`,
			);
		case "hour":
			return keepMeridiemTogether(
				Intl.DateTimeFormat("en-US", {
					hour: "numeric",
					minute: "numeric",
				}).format(date),
			);
	}
};

const getAxisDateRange = (start: Date, end: Date): "hour" | "day" | "day+year" => {
	if (differenceInHours(end, start) <= 24) return "hour";
	if (!isSameYear(start, end)) return "day+year";
	return "day";
};

const getTooltipDateRange = (
	start: Date,
	end: Date,
	range: DateRange,
): "hour" | "day+hour" | "day+hour+year" | "day" | "day+year" => {
	if (range.getGraphInterval() === "day") return getAxisDateRange(start, end) === "day+year" ? "day+year" : "day";
	if (range.value.start.toDateString() === range.value.end.toDateString()) return "hour";
	return isSameYear(start, end) ? "day+hour" : "day+hour+year";
};

const getIncompleteBucketEnd = (data: DataPoint[], range: DateRange) => {
	if (!range.endsToday() || data.length === 0) return undefined;

	const lastPoint = data[data.length - 1];
	const now = new Date();
	const bucketEnd = range.getGraphBucketEnd(lastPoint.x);
	if (now >= bucketEnd || now <= lastPoint.x) return undefined;

	return bucketEnd;
};

const pickAxisTicks = (data: DataPoint[], count: number): Date[] => {
	if (data.length <= count) return data.map((point) => point.x);

	const ticks = new Set<number>();
	for (let i = 0; i < count; i++) {
		const index = Math.round((i * (data.length - 1)) / Math.max(count - 1, 1));
		ticks.add(index);
	}

	return [...ticks].sort((a, b) => a - b).map((index) => data[index].x);
};

const getGraphRenderData = (data: DataPoint[], range: DateRange) => {
	const incompleteBucketEnd = getIncompleteBucketEnd(data, range);
	return {
		domainMaxX: data[data.length - 1]?.x ?? new Date(),
		solidLineData: incompleteBucketEnd ? data.slice(0, -1) : data,
		dottedLineData: incompleteBucketEnd
			? data.length > 1
				? [data[data.length - 2], data[data.length - 1]]
				: [data[data.length - 1]]
			: [],
	};
};

export const LineGraph = ({
	data,
	title,
	metric,
	range,
}: {
	data: DataPoint[];
	title: string;
	metric: Metric;
	range: DateRange;
}) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const containerRef = useRef<HTMLDivElement | null>(null);

	// get the container size using a resize observer
	const [dimensions, setDimensions] = useState<{
		width: number;
		height: number;
	} | null>(null);
	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		let resizeTimeout: number | undefined;
		const observer = new ResizeObserver(([entry]) => {
			if (!entry) return;
			const { width, height } = entry.contentRect;
			window.clearTimeout(resizeTimeout);
			resizeTimeout = window.setTimeout(() => {
				setDimensions((current) =>
					current?.width === width && current.height === height ? current : { width, height },
				);
			}, 100);
		});
		observer.observe(container);

		return () => {
			window.clearTimeout(resizeTimeout);
			observer.disconnect();
		};
	}, []);

	const firstRender = useRef(true);

	useEffect(() => {
		if (!svgRef.current || !dimensions) return;
		const svg = select(svgRef.current);
		const { domainMaxX, dottedLineData, solidLineData } = getGraphRenderData(data, range);

		const [minX] = extent(data, (d) => d.x).map((d) => d || new Date());
		const maxX = domainMaxX;
		const axisRange = getAxisDateRange(minX, maxX);
		const [_minY, maxY] = extent(data, (d) => d.y).map((d) => d || 0);

		let xCount = Math.min(data.length, 8);
		if (dimensions.width && dimensions.width < 400) {
			xCount = Math.min(data.length, 4);
		} else if (dimensions.width && dimensions.width < 500) {
			xCount = Math.min(data.length, 6);
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

		const svgLine = line<DataPoint>()
			.x((d) => xAxis(d.x))
			.y((d) => yAxis(d.y));

		svg
			.selectChild("#background")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgArea(data) || "");

		svg
			.selectChild("#line")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(solidLineData) || "");

		svg
			.selectChild("#line-dotted")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(dottedLineData) || "");

		const yGridElement = svg.selectChild<SVGGElement>("#y-grid");
		const yAxisElement = svg.selectChild<SVGGElement>("#y-axis");
		const xAxisElement = svg.selectChild<SVGGElement>("#x-axis");

		let tickValuesY = yAxis.ticks(5).filter((d) => d !== 0);
		// if more than 5 ticks, remove every other tick
		if (tickValuesY.length > 5) {
			tickValuesY = tickValuesY.filter((_, i) => i % 2 === 0);
		}

		const leftGridAxis = axisLeft(yAxis)
			.disableDomain()
			.tickFormat(() => "")
			.tickValues(tickValuesY);

		const leftLabelAxis = axisLeft(yAxis)
			.disableDomain()
			.disableTicks()
			.tickFormat((d) => formatMetricValEvenly(d as number, metric, maxY))
			.tickValues(tickValuesY);

		let tickValuesX = pickAxisTicks(data, xCount);
		if (tickValuesX.length > 0 && xAxis(tickValuesX[0]) < 20) tickValuesX = tickValuesX.slice(1);
		if (tickValuesX.length > 0 && xAxis(tickValuesX[tickValuesX.length - 1]) > dimensions.width - 20) {
			tickValuesX = tickValuesX.slice(0, -1);
		}

		const bottomAxis = axisBottom(xAxis)
			.disableDomain()
			.disableTicks()
			.tickFormat((d) => formatDate(d as Date, axisRange))
			.tickValues(tickValuesX);

		let xAxisTransition = 200;
		if (firstRender.current) xAxisTransition = 0;
		xAxisElement
			.interrupt()
			.transition()
			.ease(easeCubic)
			.duration(xAxisTransition)
			.call((ax) => {
				bottomAxis(ax);
				ax.attr("transform", `translate(0, ${dimensions.height - paddingBottom + 10})`);
			});

		yGridElement.call((ax) => {
			leftGridAxis(ax);
			ax.attr("transform", `translate(0, ${paddingTop})`);

			ax.selectAll(".tick line")
				.attr("x2", dimensions.width)
				.attr("stroke", "var(--pico-secondary-background)")
				.attr("stroke-width", 0.5)
				.attr("stroke-dasharray", "2, 2");

			ax.selectAll("text").remove();
		});

		yAxisElement.call((ax) => {
			leftLabelAxis(ax);
			ax.attr("transform", `translate(0, ${paddingTop})`);

			// move the labels to the right side of the graph, keeping the text right-aligned
			ax.selectAll(".tick text")
				.attr("x", dimensions.width - 35)
				.attr("text-anchor", "end")
				.attr("dx", "2em")
				.attr("dy", "0.3em");

			// remove existing blocks
			ax.selectAll("rect").remove();

			// add black blocks before the labels with the same width as the text
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

			firstRender.current = false;
		});

		return () => void xAxisElement.interrupt();
	}, [data, dimensions, metric, range]);

	useEffect(() => {
		const svgElement = svgRef.current;
		if (!svgElement || !dimensions || data.length === 0) return;

		const svg = select(svgElement);
		const tooltip = svg.selectChild("#tooltip");
		const needle = svg.selectChild("#needle");
		const { domainMaxX } = getGraphRenderData(data, range);
		const [minX] = extent(data, (d) => d.x).map((d) => d || new Date());
		const xAxis = scaleTime().domain([minX, domainMaxX]).range([0, dimensions.width]);
		const dateRange = getTooltipDateRange(minX, domainMaxX, range);
		let animationFrame: number | undefined;

		const mouseMove = (event: MouseEvent) => {
			window.cancelAnimationFrame(animationFrame ?? 0);
			animationFrame = window.requestAnimationFrame(() => {
				const tooltipRect = (tooltip.node() as SVGForeignObjectElement | null)?.getBoundingClientRect();
				const tooltipWidth = tooltipRect?.width || 0;
				const tooltipHeight = tooltipRect?.height || 0;
				const tooltipPadding = 10;

				const svgRect = svgElement.getBoundingClientRect();
				const svgWidth = svgRect.width;
				const svgHeight = svgRect.height;
				const isLeftSide = event.clientX - svgRect.left < svgWidth / 2;
				const tooltipX = isLeftSide
					? Math.min(event.clientX - svgRect.left + tooltipPadding, svgWidth - tooltipWidth - tooltipPadding)
					: Math.max(event.clientX - svgRect.left - tooltipWidth - tooltipPadding, tooltipPadding);
				const tooltipY = Math.min(
					event.clientY - svgRect.top + tooltipPadding - tooltipHeight / 3,
					svgHeight - tooltipHeight - tooltipPadding,
				);

				tooltip.attr("x", tooltipX).attr("y", tooltipY).attr("opacity", 1);

				const x = event.clientX - svgRect.left - 1;
				const point = data.reduce((closestPoint, currentPoint) => {
					const closestDistance = Math.abs(xAxis(closestPoint.x) - x);
					const currentDistance = Math.abs(xAxis(currentPoint.x) - x);
					return currentDistance < closestDistance ? currentPoint : closestPoint;
				});

				const snappedX = xAxis(point.x);
				needle.attr("d", `M ${snappedX} 0 L ${snappedX} ${svgHeight - 40}`);
				tooltip.select(".date").text(formatDate(new Date(point.x), dateRange));
				tooltip.select(".value").text(formatMetricVal(point.y, metric));
			});
		};

		const mouseLeave = () => {
			window.cancelAnimationFrame(animationFrame ?? 0);
			tooltip.interrupt().attr("opacity", 0);
			needle.interrupt().attr("d", "M 0 0 L 0 0");
		};

		svgElement.addEventListener("mousemove", mouseMove);
		svgElement.addEventListener("mouseleave", mouseLeave);

		return () => {
			window.cancelAnimationFrame(animationFrame ?? 0);
			svgElement.removeEventListener("mousemove", mouseMove);
			svgElement.removeEventListener("mouseleave", mouseLeave);
			tooltip.interrupt();
			needle.interrupt();
		};
	}, [data, dimensions, metric, range]);

	return (
		<div ref={containerRef} className={styles.graph}>
			<svg
				ref={svgRef}
				style={{ display: "block", width: "100%", height: "100%" }}
				role="img"
				aria-label={`${title} time series`}
			>
				<title>{title} graph</title>
				<defs>
					<linearGradient id="graphGradient" x1="0" x2="0" y1="0" y2="1">
						<stop offset="0%" stopColor="rgb(var(--graph-fill-color) / 0.25)" />
						<stop offset="100%" stopColor="rgb(var(--graph-fill-color) / 0)" />
					</linearGradient>
				</defs>
				<g id="y-grid" />
				<path id="background" fill="url(#graphGradient)" stroke="none" />
				<path id="line" fill="none" stroke="var(--graph-line-color)" />
				<path id="line-dotted" fill="none" stroke="var(--graph-line-color)" strokeDasharray="5, 5" />
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
						</h3>
					</div>
				</foreignObject>
				<g id="y-axis" />
				<g id="x-axis" />
			</svg>
		</div>
	);
};
