import styles from "./project.module.css";
import _map from "./worldmap/map.module.css";

import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";

import { metricNames, useDimension, useProject, useProjectGraph, useProjectStats } from "../api";
import type { Dimension, DimensionFilter, DimensionTableRow, Metric, ProjectResponse } from "../api";
import type { DateRange } from "../api/ranges";

import { useMetric, useRange } from "../hooks/persist";
import { cls } from "../utils";
import { DimensionCard, DimensionDropdownCard, DimensionTabs, DimensionTabsCard, cardStyles } from "./dimensions";
import { LineGraph } from "./graph";
// import { LineGraph2 } from "./graph2";
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

	const { project } = useProject(projectId);
	const { graph } = useProjectGraph({ projectId, metric, range, filters });
	const { stats } = useProjectStats({ projectId, metric, range, filters });

	const query = useMemo<ProjectQuery>(
		// biome-ignore lint/style/noNonNullAssertion: this is safe because code using this query will only run when project is defined.
		() => ({ project: project!, metric, range, filters }),
		[project, metric, range, filters],
	);

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

	if (!project) return null;

	return (
		<div className={styles.project}>
			<Suspense fallback={null}>
				<div>
					<div className={styles.projectHeader}>
						<ProjectHeader project={project} stats={stats} />
						<SelectRange onSelect={setRange} range={range} projectId={project.id} />
					</div>
					<SelectMetrics data={stats} metric={metric} setMetric={setMetric} className={styles.projectStats} />
					<SelectFilters value={filters} onChange={setFilters} />
				</div>
				<article className={cls(cardStyles, styles.graphCard)}>
					<LineGraph data={graph ?? []} metric={metric} title={metricNames[metric]} range={range} />
				</article>
				{/* <article className={cls(cardStyles, styles.graphCard2)}>
					<LineGraph2 data={graph ?? []} metric={metric} title={metricNames[metric]} range={range} />
				</article> */}
				<div className={styles.tables}>
					<DimensionTabsCard dimensions={["url", "fqdn"]} query={query} onSelect={onSelectDimRow} />
					<DimensionDropdownCard
						dimensions={["referrer", "utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"]}
						query={query}
						onSelect={onSelectDimRow}
					/>
					<GeoCard query={query} onSelect={onSelectDimRow} />
					<DimensionTabsCard dimensions={["platform", "browser"]} query={query} onSelect={onSelectDimRow} />
					<DimensionCard dimension={"mobile"} query={query} onSelect={(v) => onSelectDimRow(v, "mobile")} />
				</div>
			</Suspense>
		</div>
	);
};

const GeoCard = ({
	query,
	onSelect,
}: {
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	const { data } = useDimension({
		dimension: "country",
		...query,
	});

	return (
		<article className={cls(cardStyles, styles.geoCard)} data-full-width="true">
			<div className={styles.geoMap}>
				<Suspense fallback={null}>
					<Worldmap data={data} metric={query.metric} />
				</Suspense>
			</div>
			<div className={styles.geoTable}>
				<DimensionTabs dimensions={["country", "city"]} query={query} onSelect={onSelect} />
			</div>
		</article>
	);
};
