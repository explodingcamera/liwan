import styles from "./project.module.css";
import cardStyles from "./dimensions/dimensions.module.css";

import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";

import { dimensions, metrics, metricNames, useDimension, useProject, useProjectGraph, useProjectStats } from "../api";
import type { Dimension, DimensionFilter, DimensionTableRow, Metric, ProjectResponse } from "../api";
import type { DateRange } from "../api/ranges";

import { useMetric, useRange } from "../hooks/persist";
import { cls } from "../utils";
import { DimensionDropdownCard, DimensionTabs, DimensionTabsCard, PageDimensionTabsCard } from "./dimensions";
import { LineGraph } from "./graph";
import { SelectFilters } from "./project/filter";
import { SelectMetrics } from "./project/metric";
import { ProjectHeader } from "./project/project";
import { SelectRange } from "./project/range";

const Worldmap = lazy(() => import("./worldmap").then((module) => ({ default: module.Worldmap })));
export type ProjectQuery = {
	project: ProjectResponse;
	metric: Metric;
	range: DateRange;
	filters: DimensionFilter[];
};

const getDimensionFilter = (dimension: Dimension, value: string): DimensionFilter => {
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
	const [projectId, setProjectId] = useState<string | undefined>();
	const [filters, setFilters] = useState<DimensionFilter[]>([]);

	const { metric, setMetric } = useMetric();
	const { range, setRange } = useRange();

	useEffect(() => {
		if (typeof window === "undefined") return;
		setProjectId(window?.document.location.pathname.split("/").pop());
	}, []);

	const { project, notFound } = useProject(projectId);
	const visibleMetrics = useMemo(
		() => metrics.filter((item) => !project?.hiddenMetrics.includes(item)),
		[project?.hiddenMetrics],
	);
	const activeMetric = visibleMetrics.includes(metric) ? metric : visibleMetrics[0];
	const reportMetric = activeMetric ?? "views";
	const visibleFilters = useMemo(
		() => filters.filter((filter) => !project?.hiddenDimensions.includes(filter.dimension)),
		[filters, project?.hiddenDimensions],
	);
	const {
		graph,
		isUpdating: graphUpdating,
		isLoading: graphLoading,
	} = useProjectGraph({
		projectId,
		metric: reportMetric,
		range,
		filters: visibleFilters,
		enabled: Boolean(activeMetric),
	});
	const { stats } = useProjectStats({
		projectId,
		metric: reportMetric,
		range,
		filters: visibleFilters,
		enabled: Boolean(activeMetric),
	});

	const query = useMemo<ProjectQuery>(
		// biome-ignore lint/style/noNonNullAssertion: this is safe because code using this query will only run when project is defined.
		() => ({ project: project!, metric: reportMetric, range, filters: visibleFilters }),
		[project, reportMetric, range, visibleFilters],
	);

	useEffect(() => {
		if (activeMetric && activeMetric !== metric) setMetric(activeMetric);
	}, [activeMetric, metric, setMetric]);

	const toggleFilter = useCallback(
		(filter: DimensionFilter) => {
			const index = filters.findIndex((f) => f.dimension === filter.dimension && f.filterType === filter.filterType);
			if (index === -1) {
				setFilters([...filters, filter]);
			} else {
				setFilters(filters.filter((_, i) => i !== index));
			}
		},
		[filters],
	);

	const onSelectDimRow = useCallback(
		(value: DimensionTableRow, dimension: Dimension) => {
			toggleFilter(getDimensionFilter(dimension, value.dimensionValue));
		},
		[toggleFilter],
	);

	if (notFound) {
		return <div className={styles.notFound}>Project not found</div>;
	}

	if (!project) return null;
	const visibleDimensions = (items: Dimension[]) =>
		items.filter((dimension) => !project.hiddenDimensions.includes(dimension));
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
			<Suspense fallback={null}>
				<div>
					<div className={styles.projectHeader}>
						<ProjectHeader project={project} stats={stats} />
						<SelectRange onSelect={setRange} range={range} projectId={project.id} />
					</div>
					<SelectMetrics
						data={stats}
						metric={reportMetric}
						metrics={visibleMetrics}
						setMetric={setMetric}
						className={styles.projectStats}
					/>
					<SelectFilters
						value={visibleFilters}
						onChange={setFilters}
						dimensions={dimensions.filter((dimension) => !project.hiddenDimensions.includes(dimension))}
					/>
				</div>
				<article className={cls(cardStyles.card, styles.graphCard)}>
					{activeMetric ? (
						<LineGraph
							data={graph}
							title={metricNames[reportMetric]}
							metric={reportMetric}
							range={range}
							isLoading={graphLoading}
							isUpdating={graphUpdating}
						/>
					) : (
						<div className={styles.emptyReport}>No metrics are visible for this project.</div>
					)}
				</article>
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
				</div>
			</Suspense>
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
