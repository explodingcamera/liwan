import tooltipStyles from "../hover-tooltip.module.css";
import styles from "./linegraph.module.css";

import { useEffect, useMemo, useRef, useState } from "react";
import { bisector, max } from "d3-array";
import { easeCubic, easeLinear } from "d3-ease";
import { scaleLinear, scaleTime } from "d3-scale";
import { select } from "d3-selection";
import { area, line } from "d3-shape";
import "d3-transition";

import type { DateRange } from "@/api/ranges";
import type { GraphInterval, Metric } from "@/constants";
import type { ComparisonRange } from "../comparison";
import { formatMetricVal } from "../metric-format";
import type { DataPoint } from ".";
import { axisBottom, axisLeft } from "./axis";
import { formatDate, getAxisDateRange, getTooltipDateRange } from "./format";
import {
	alignComparisonData,
	type ComparisonPoint,
	getGraphRenderData,
	getGraphSelectionIndices,
	getSelectedGraphRange,
} from "./render-data";

const pickAxisTicks = (data: { x: Date }[], count: number): Date[] => {
	if (data.length <= count) return data.map((point) => point.x);

	return Array.from({ length: count }, (_, i) => data[Math.round((i * (data.length - 1)) / (count - 1))].x);
};

const nearestBucket = bisector((point: { x: Date }) => point.x.getTime()).center;

export const LineGraph = ({
	data,
	title,
	metric,
	range,
	interval,
	hasResolutionControl,
	canSelectRange,
	onSelectRange,
	onUndoRange,
	comparisonData,
	comparisonRange,
}: {
	data: DataPoint[];
	title: string;
	metric: Metric;
	range: DateRange;
	interval: GraphInterval;
	hasResolutionControl: boolean;
	canSelectRange: boolean;
	onSelectRange: (range: DateRange) => void;
	onUndoRange: () => void;
	comparisonData?: DataPoint[];
	comparisonRange?: ComparisonRange;
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
	const extendBy = comparisonRange?.fullPeriod ? interval : undefined;
	const comparison = useMemo(
		() => alignComparisonData(data, comparisonData, extendBy),
		[data, comparisonData, extendBy],
	);
	const axisData = comparison.length > data.length ? comparison : data;

	useEffect(() => {
		if (!svgRef.current || !dimensions) return;
		const svg = select(svgRef.current);
		const { dottedLineData, solidLineData } = getGraphRenderData(data, range, interval);

		const minX = axisData[0]?.x ?? range.value.start;
		const maxX = axisData.at(-1)?.x ?? minX;
		const axisRange = getAxisDateRange(minX, maxX);
		const maxY = Math.max(max(data, (point) => point.y) ?? 0, max(comparison, (point) => point.y) ?? 0);
		const xCount = dimensions.width < 400 ? 4 : dimensions.width < 500 ? 6 : 8;

		const paddingTop = hasResolutionControl ? 48 : 30;
		const paddingBottom = 40;

		const xAxis = scaleTime().domain([minX, maxX]).range([0, dimensions.width]);

		const yAxis =
			metric === "bounce_rate"
				? scaleLinear([0, 1], [dimensions.height - paddingBottom - paddingTop, 0])
				: scaleLinear([0, Math.max(maxY * 1.25 || 0, 20)], [dimensions.height - paddingBottom - paddingTop, 0]);

		const svgArea = area<DataPoint | ComparisonPoint>()
			.defined((d) => d.y !== null)
			.x((d) => xAxis(d.x))
			// Extend the fill behind the date labels without changing the plotted scale.
			.y0(dimensions.height - paddingTop)
			.y1((d) => yAxis(d.y ?? 0));

		const svgLine = line<DataPoint | ComparisonPoint>()
			.defined((d) => d.y !== null)
			.x((d) => xAxis(d.x))
			.y((d) => yAxis(d.y ?? 0));

		const currentEnd = data.at(-1)?.x;
		const remainingPeriod =
			currentEnd && currentEnd < maxX
				? [
						{ x: currentEnd, y: 0 },
						{ x: maxX, y: 0 },
					]
				: [];

		svg
			.selectChild("#background")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", (svgArea(data) ?? "") + (svgArea(remainingPeriod) ?? ""));

		svg
			.selectChild("#line")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(solidLineData) || "");

		svg
			.selectChild("#line-dotted")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(dottedLineData) || "");

		svg
			.selectChild("#remaining-period")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(remainingPeriod) ?? "");

		const yGridElement = svg.selectChild<SVGGElement>("#y-grid");
		svg
			.selectChild("#comparison-line")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgLine(comparison) || "");
		svg
			.selectChild("#comparison-background")
			.attr("transform", `translate(0, ${paddingTop})`)
			.attr("d", svgArea(comparison) || "");
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
			.tickFormat((d) => formatMetricVal(d as number, metric, maxY))
			.tickValues(tickValuesY);

		let tickValuesX = pickAxisTicks(axisData, xCount);
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
				.attr("stroke", "var(--chart-grid)")
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
					.attr("fill", "var(--surface)");
			});

			firstRender.current = false;
		});

		return () => void xAxisElement.interrupt();
	}, [data, comparison, axisData, dimensions, metric, range, interval, hasResolutionControl]);

	useEffect(() => {
		const svgElement = svgRef.current;
		if (!svgElement || !dimensions || data.length === 0) return;

		const svg = select(svgElement);
		const cursor = svg.selectChild("#cursor");
		const selection = svg.selectChild("#selection");
		const tooltip = cursor.selectChild("#tooltip");
		const needle = cursor.selectChild("#needle");
		const tooltipElement = tooltip.node() as SVGForeignObjectElement | null;
		const tooltipHeight = Number(tooltipElement?.getAttribute("height")) || 100;
		const minX = axisData[0].x;
		const maxX = axisData[axisData.length - 1].x;
		const xAxis = scaleTime().domain([minX, maxX]).range([0, dimensions.width]);
		const dateRange = getTooltipDateRange(minX, maxX, range, interval, comparisonRange);
		let animationFrame: number | undefined;
		let previousSide: "left" | "right" | undefined;
		let previousOffset: number | undefined;
		let previousSnappedX: number | undefined;
		let dragStart: number | undefined;
		let dragPointerId: number | undefined;
		const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
		const pointerX = (event: PointerEvent) =>
			Math.max(0, Math.min(event.clientX - svgElement.getBoundingClientRect().left, dimensions.width));
		const pointerDown = (event: PointerEvent) => {
			if (!canSelectRange || data.length < 2 || event.pointerType !== "mouse" || event.button !== 0) return;
			event.preventDefault();
			window.cancelAnimationFrame(animationFrame ?? 0);
			dragStart = pointerX(event);
			dragPointerId = event.pointerId;
			svgElement.setPointerCapture(event.pointerId);
		};
		const pointerMove = (event: PointerEvent) => {
			if (dragStart === undefined) return;
			const dragEnd = pointerX(event);
			if (Math.abs(dragEnd - dragStart) < 6) {
				selection.attr("visibility", "hidden");
				return;
			}
			cursor.attr("opacity", 0);
			const indices = getGraphSelectionIndices(data, xAxis.invert(dragStart), xAxis.invert(dragEnd));
			if (!indices) return;
			const [first, last] = indices;
			const left = xAxis(data[first].x);
			const right = xAxis(data[last].x);
			selection
				.selectChild("rect")
				.attr("x", Math.min(left, dimensions.width - 2))
				.attr("width", Math.max(2, right - left))
				.attr("height", Math.max(0, dimensions.height - 40));
			selection
				.selectChild("#selection-start")
				.attr("x1", left)
				.attr("x2", left)
				.attr("y2", dimensions.height - 40);
			selection
				.selectChild("#selection-end")
				.attr("x1", right)
				.attr("x2", right)
				.attr("y2", dimensions.height - 40);
			selection.attr("visibility", "visible");
		};
		const cancelDrag = () => {
			const pointerId = dragPointerId;
			dragStart = undefined;
			dragPointerId = undefined;
			selection.attr("visibility", "hidden");
			if (pointerId !== undefined && svgElement.hasPointerCapture(pointerId))
				svgElement.releasePointerCapture(pointerId);
		};
		const pointerUp = (event: PointerEvent) => {
			if (dragStart === undefined) return;
			const end = pointerX(event);
			const start = dragStart;
			cancelDrag();
			if (Math.abs(end - start) < 6) return;
			const selected = getSelectedGraphRange(data, range, interval, xAxis.invert(start), xAxis.invert(end));
			if (selected) onSelectRange(selected);
		};
		const doubleClick = () => {
			if (canSelectRange) onUndoRange();
		};
		const keyDown = (event: KeyboardEvent) => {
			if (event.key !== "Escape" || dragStart === undefined) return;
			event.preventDefault();
			cancelDrag();
		};

		const mouseMove = (event: MouseEvent) => {
			if (dragStart !== undefined) return;
			window.cancelAnimationFrame(animationFrame ?? 0);
			animationFrame = window.requestAnimationFrame(() => {
				const tooltipPadding = 10;

				const svgRect = svgElement.getBoundingClientRect();
				const svgWidth = svgRect.width;
				const svgHeight = svgRect.height;
				const tooltipWidth = Math.min(
					comparisonData ? 260 : metric === "unique_visitors" || metric === "avg_time_on_site" ? 232 : 190,
					Math.max(0, svgWidth - tooltipPadding * 2),
				);
				tooltipElement?.setAttribute("width", String(tooltipWidth));
				const x = event.clientX - svgRect.left - 1;
				const index = nearestBucket(axisData, xAxis.invert(x).getTime());
				const point = axisData[index];

				const snappedX = xAxis(point.x);
				const side = snappedX < svgWidth / 2 ? "right" : "left";
				const tooltipX = side === "right" ? snappedX + tooltipPadding : snappedX - tooltipWidth - tooltipPadding;
				const clampedX = Math.max(tooltipPadding, Math.min(tooltipX, svgWidth - tooltipWidth - tooltipPadding));
				const offset = clampedX - snappedX;
				const tooltipY = Math.max(
					tooltipPadding,
					Math.min(
						event.clientY - svgRect.top + tooltipPadding - tooltipHeight / 3,
						svgHeight - tooltipHeight - tooltipPadding,
					),
				);
				const wasVisible = cursor.attr("opacity") !== "0";
				tooltip.attr("y", tooltipY);
				if (wasVisible && previousSide && side !== previousSide && !reducedMotion) {
					tooltip
						.interrupt()
						.transition()
						.duration(90)
						.attr("opacity", 0)
						.on("end", () => {
							tooltip.attr("x", offset).transition().duration(130).attr("opacity", 1);
						});
				} else if (!wasVisible || offset !== previousOffset) {
					tooltip.interrupt().attr("x", offset).attr("opacity", 1);
				}
				previousSide = side;
				previousOffset = offset;
				needle.attr("d", `M 0 0 L 0 ${svgHeight - 40}`);
				if (!wasVisible || snappedX !== previousSnappedX) {
					cursor.interrupt().attr("opacity", 1);
					if (wasVisible && !reducedMotion) {
						cursor.transition().duration(70).ease(easeLinear).attr("transform", `translate(${snappedX}, 0)`);
					} else {
						cursor.attr("transform", `translate(${snappedX}, 0)`);
					}
				}
				previousSnappedX = snappedX;
				tooltip.select(".date").text(formatDate(new Date(point.x), dateRange));
				tooltip.select(".value").text(formatMetricVal(data[index]?.y, metric));
				const previous = comparison[index];
				tooltip.select(".comparison-date").text(previous?.date ? formatDate(previous.date, dateRange) : "Comparison");
				tooltip.select(".comparison-value").text(formatMetricVal(previous?.y, metric));
			});
		};

		const mouseLeave = () => {
			window.cancelAnimationFrame(animationFrame ?? 0);
			cursor.interrupt().attr("opacity", 0);
			tooltip.interrupt().attr("opacity", 1);
			previousSide = undefined;
			previousOffset = undefined;
			previousSnappedX = undefined;
		};

		svgElement.addEventListener("mousemove", mouseMove);
		svgElement.addEventListener("mouseleave", mouseLeave);
		svgElement.addEventListener("pointerdown", pointerDown);
		svgElement.addEventListener("pointermove", pointerMove);
		svgElement.addEventListener("pointerup", pointerUp);
		svgElement.addEventListener("pointercancel", cancelDrag);
		svgElement.addEventListener("lostpointercapture", cancelDrag);
		svgElement.addEventListener("dblclick", doubleClick);
		window.addEventListener("keydown", keyDown);

		return () => {
			window.cancelAnimationFrame(animationFrame ?? 0);
			svgElement.removeEventListener("mousemove", mouseMove);
			svgElement.removeEventListener("mouseleave", mouseLeave);
			svgElement.removeEventListener("pointerdown", pointerDown);
			svgElement.removeEventListener("pointermove", pointerMove);
			svgElement.removeEventListener("pointerup", pointerUp);
			svgElement.removeEventListener("pointercancel", cancelDrag);
			svgElement.removeEventListener("lostpointercapture", cancelDrag);
			svgElement.removeEventListener("dblclick", doubleClick);
			window.removeEventListener("keydown", keyDown);
			cancelDrag();
			cursor.interrupt();
			tooltip.interrupt();
			cursor.attr("opacity", 0);
			tooltip.attr("opacity", 1);
		};
	}, [
		data,
		comparison,
		axisData,
		comparisonData,
		comparisonRange,
		dimensions,
		metric,
		range,
		interval,
		onSelectRange,
		onUndoRange,
		canSelectRange,
	]);

	return (
		<div ref={containerRef} className={styles.graph}>
			<svg
				ref={svgRef}
				style={{ display: "block", width: "100%", height: "100%" }}
				role="img"
				aria-label={`${title} time series${comparisonData ? " with comparison period" : ""}`}
			>
				<g id="y-grid" />
				<path id="comparison-background" fill="var(--chart-fill)" fillOpacity="0.3" stroke="none" />
				<path id="comparison-line" fill="none" stroke="var(--chart-comparison-line)" strokeLinecap="round" />
				<path id="background" fill="var(--chart-fill)" stroke="none" />
				<path id="line" fill="none" stroke="var(--chart-line)" strokeWidth="1.75" />
				<path id="line-dotted" fill="none" stroke="var(--chart-line)" strokeWidth="1.75" strokeDasharray="5, 5" />
				<path id="remaining-period" fill="none" stroke="var(--chart-line)" strokeOpacity="0.5" strokeDasharray="5, 5" />
				<g id="selection" className={styles.selection} visibility="hidden">
					<rect y="0" />
					<line id="selection-start" y1="0" stroke="var(--accent-fill)" strokeDasharray="5, 5" strokeWidth="2" />
					<line id="selection-end" y1="0" stroke="var(--accent-fill)" strokeDasharray="5, 5" strokeWidth="2" />
				</g>
				<g id="cursor" opacity="0">
					<path id="needle" fill="none" stroke="var(--accent-fill)" strokeDasharray="5, 5" strokeWidth="2" />
					<foreignObject id="tooltip" width="260" height={comparisonData ? "155" : "100"}>
						<div className={`${tooltipStyles.tooltip} ${tooltipStyles.graphTooltip}`}>
							<h2 className="date">{title}</h2>
							<h3>
								<span>{title}</span> <span className="value" />
							</h3>
							{comparisonData && (
								<div className={styles.comparisonTooltip}>
									<h2 className="comparison-date">Comparison</h2>
									<h3>
										<span>{title}</span>
										<span className="comparison-value" />
									</h3>
								</div>
							)}
						</div>
					</foreignObject>
				</g>
				<g id="y-axis" />
				<g id="x-axis" />
			</svg>
		</div>
	);
};
