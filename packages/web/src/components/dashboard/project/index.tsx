import cardStyles from "./dimensions/dimensions.module.css";
import styles from "./index.module.css";

import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { DateRange } from "@/api/ranges";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Dimension, DimensionFilter, DimensionTableRow, Metric, ProjectResponse } from "@/constants";
import { dimensions, eventMetricName, metrics } from "@/constants";
import { useDimension, useProject, useProjectGraph, useProjectStats } from "@/hooks/api";
import { useMetric, useRange } from "@/hooks/persist";
import { cls } from "@/utils";
import { CustomEventsCard } from "./custom-events";
import { DimensionDropdownCard, DimensionTabs, DimensionTabsCard, PageDimensionTabsCard } from "./dimensions";
import { SelectFilters } from "./filter";
import { LineGraph } from "./graph";
import { SelectMetrics } from "./metric";
import { ProjectHeader } from "./project-header";
import { PropertiesCard } from "./properties";
import { SelectRange } from "./range";

const Worldmap = lazy(() => import("./worldmap").then((module) => ({ default: module.Worldmap })));
export type ProjectQuery = {
	project: ProjectResponse;
	metric: Metric;
	range: DateRange;
	filters: DimensionFilter[];
	eventName: string;
};

export const getDimensionFilter = (dimension: Dimension, value: string, propertyKey?: string): DimensionFilter => {
	if (dimension === "property") return { dimension, key: propertyKey, filterType: "equal", value };

	if (dimension === "city")
		// remove the first two characters from the dimension value
		// which are the country code
		return {
			dimension: "city",
			filterType: "equal",
			value: value.slice(2),
		};

	if (dimension === "mobile")
		return {
			dimension: "mobile",
			filterType: value === "true" ? "is_true" : "is_false",
		};

	if (value === "Unknown")
		return {
			dimension,
			filterType: "is_null",
		};

	return {
		dimension,
		filterType: "equal",
		value: value,
	};
};

export const Project = () => {
	const [projectId] = useState(() =>
		typeof window === "undefined" ? undefined : window.location.pathname.split("/").pop(),
	);
	const [filters, setFilters] = useState<DimensionFilter[]>([]);
	const [eventName, setEventName] = useState("pageview");

	const { metric, setMetric } = useMetric();
	const { range, setRange, interval: graphInterval, setInterval: setGraphInterval } = useRange();
	const previousGraphRange = useRef<{ before: DateRange; after: DateRange; interval: typeof graphInterval } | null>(
		null,
	);
	const selectRange = useCallback(
		(selected: DateRange) => {
			previousGraphRange.current = null;
			setRange(selected);
		},
		[setRange],
	);
	const selectGraphRange = useCallback(
		(selected: DateRange) => {
			const origin =
				previousGraphRange.current?.after.serialize() === range.serialize() ? previousGraphRange.current : null;
			previousGraphRange.current = {
				before: origin?.before ?? range,
				after: selected,
				interval: origin?.interval ?? graphInterval,
			};
			setRange(selected);
			setGraphInterval("auto");
		},
		[range, graphInterval, setRange, setGraphInterval],
	);
	const undoGraphRange = useCallback(() => {
		if (previousGraphRange.current?.after.serialize() !== range.serialize()) return;
		setRange(previousGraphRange.current.before);
		setGraphInterval(previousGraphRange.current.interval);
		previousGraphRange.current = null;
	}, [range, setRange, setGraphInterval]);

	const { project, notFound } = useProject(projectId);
	const visibleMetrics: Metric[] = useMemo(
		() =>
			metrics.filter(
				(item) =>
					!project?.hiddenMetrics.includes(item) &&
					(eventName === "pageview" || item === "views" || item === "unique_visitors"),
			),
		[project?.hiddenMetrics, eventName],
	);
	const activeMetric = visibleMetrics.includes(metric) ? metric : visibleMetrics[0];
	const reportMetric: Metric = activeMetric ?? "views";
	const visibleFilters = useMemo(
		() =>
			filters.filter(
				(filter) =>
					!project?.hiddenDimensions.includes(filter.dimension) &&
					(eventName === "pageview" || (filter.dimension !== "url_entry" && filter.dimension !== "url_exit")),
			),
		[filters, project?.hiddenDimensions, eventName],
	);
	const {
		graph,
		displayMetric,
		displayRange,
		displayInterval,
		isUpdating: graphUpdating,
		isLoading: graphLoading,
	} = useProjectGraph({
		projectId,
		metric: reportMetric,
		range,
		filters: visibleFilters,
		eventName,
		enabled: Boolean(activeMetric),
		interval: graphInterval,
	});
	const {
		stats,
		isLoading: statsLoading,
		isUpdating: statsUpdating,
	} = useProjectStats({
		projectId,
		range,
		filters: visibleFilters,
		eventName,
		enabled: Boolean(activeMetric),
	});

	const query = useMemo<ProjectQuery>(
		() => ({
			// biome-ignore lint/style/noNonNullAssertion: this is safe because code using this query will only run when project is defined.
			project: project!,
			metric: reportMetric,
			range,
			filters: visibleFilters,
			eventName,
		}),
		[project, reportMetric, range, visibleFilters, eventName],
	);

	useEffect(() => {
		if (eventName === "pageview" && activeMetric && activeMetric !== metric) setMetric(activeMetric);
	}, [activeMetric, eventName, metric, setMetric]);

	const toggleFilter = useCallback(
		(filter: DimensionFilter) => {
			const index = filters.findIndex(
				(f) => f.dimension === filter.dimension && f.key === filter.key && f.filterType === filter.filterType,
			);
			if (index === -1) {
				setFilters([...filters, filter]);
			} else if (filters[index].value !== filter.value || filters[index].inversed || filters[index].strict) {
				setFilters(filters.map((current, i) => (i === index ? filter : current)));
			} else {
				setFilters(filters.filter((_, i) => i !== index));
			}
		},
		[filters],
	);

	const selectEvent = (name: string) => {
		setEventName(name);
		if (name !== "pageview") {
			setFilters((current) =>
				current.filter((filter) => filter.dimension !== "url_entry" && filter.dimension !== "url_exit"),
			);
		}
	};

	const onSelectDimRow = useCallback(
		(value: DimensionTableRow, dimension: Dimension) => {
			toggleFilter(getDimensionFilter(dimension, value.dimensionValue));
		},
		[toggleFilter],
	);

	if (notFound) {
		return <div className={styles.notFound}>Project not found</div>;
	}

	if (!project)
		return (
			<div className={styles.loading} role="status" aria-label="Loading project">
				<LoadingSpinner />
			</div>
		);
	const visibleDimensions = (items: Dimension[]) =>
		items.filter(
			(dimension) =>
				!project.hiddenDimensions.includes(dimension) &&
				(eventName === "pageview" || (dimension !== "url_entry" && dimension !== "url_exit")),
		);
	const pageDimensions = visibleDimensions(["url", "url_entry", "url_exit", "fqdn"]);
	const campaignDimensions = visibleDimensions([
		"referrer",
		"utm_source",
		"utm_medium",
		"utm_campaign",
		"utm_content",
		"utm_term",
	]);
	const geoDimensions = visibleDimensions(["country", "city"]);
	const technologyDimensions = visibleDimensions(["platform", "browser"]);
	const deviceDimensions = visibleDimensions(["mobile", "screen_width", "orientation"]);

	return (
		<div className={styles.project}>
			<div className={styles.projectReport}>
				<div className={styles.projectHeader}>
					<ProjectHeader project={project} stats={stats} />
					<SelectRange onSelect={selectRange} range={range} projectId={project.id} />
				</div>
				<SelectMetrics
					className={styles.projectStats}
					data={stats}
					metric={reportMetric}
					metrics={visibleMetrics}
					setMetric={setMetric}
					isLoading={statsLoading || statsUpdating}
					eventName={eventName}
				/>
				<SelectFilters
					eventName={eventName}
					onClearEvent={() => selectEvent("pageview")}
					value={visibleFilters}
					onChange={setFilters}
					dimensions={[...dimensions, "property" as const].filter(
						(dimension) =>
							!project.hiddenDimensions.includes(dimension) &&
							(eventName === "pageview" || (dimension !== "url_entry" && dimension !== "url_exit")),
					)}
				/>
				<article className={cls(cardStyles.card, styles.graphCard)}>
					{activeMetric ? (
						<LineGraph
							data={graph}
							title={eventMetricName(displayMetric, eventName)}
							metric={displayMetric}
							range={displayRange}
							interval={displayInterval}
							selectedInterval={graphInterval}
							onSelectInterval={setGraphInterval}
							onSelectRange={graphLoading || graphUpdating ? undefined : selectGraphRange}
							onUndoRange={undoGraphRange}
							availableIntervals={range.getGraphIntervals()}
							isLoading={graphLoading}
							isUpdating={graphUpdating}
						/>
					) : (
						<div className={styles.emptyReport}>No metrics are visible for this project.</div>
					)}
				</article>
			</div>
			<div className={styles.tables}>
				{activeMetric && pageDimensions.length > 0 && (
					<PageDimensionTabsCard dimensions={pageDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				{activeMetric && campaignDimensions.length > 0 && (
					<DimensionDropdownCard dimensions={campaignDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				{activeMetric && geoDimensions.includes("country") && (
					<GeoCard dimensions={geoDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				{activeMetric && geoDimensions.length > 0 && !geoDimensions.includes("country") && (
					<DimensionTabsCard dimensions={geoDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				{activeMetric && technologyDimensions.length > 0 && (
					<DimensionTabsCard dimensions={technologyDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				{activeMetric && deviceDimensions.length > 0 && (
					<DimensionDropdownCard dimensions={deviceDimensions} query={query} onSelect={onSelectDimRow} />
				)}
				<CustomEventsCard query={query} onSelectEvent={selectEvent} />
				{activeMetric && !project.hiddenDimensions.includes("property") && (
					<PropertiesCard query={query} onSelect={toggleFilter} setFilters={setFilters} />
				)}
			</div>
		</div>
	);
};

const GeoCard = ({
	dimensions,
	query,
	onSelect,
}: {
	dimensions: Dimension[];
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	const { data } = useDimension({
		dimension: "country",
		...query,
	});

	return (
		<article className={cls(cardStyles.card, styles.geoCard, "geocard")} data-full-width="true">
			<div className={styles.geoMap}>
				<Suspense fallback={null}>
					<Worldmap data={data} metric={query.metric} />
				</Suspense>
			</div>
			<div className={styles.geoTable}>
				<DimensionTabs dimensions={dimensions} query={query} onSelect={onSelect} />
			</div>
		</article>
	);
};
