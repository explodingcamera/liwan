import styles from "./project.module.css";
import _map from "./worldmap.module.css";

import { Suspense, lazy, useEffect, useState } from "react";
import { useLocalStorage } from "@uidotdev/usehooks";

import { type RangeName, resolveRange } from "../api/ranges";
import { metricNames, useDimension, useProject, useProjectData } from "../api";
import type { DimensionFilter, DateRange, Metric, ProjectResponse, DimensionTableRow, Dimension } from "../api";

import { cls } from "../utils";
import { LineGraph } from "./graph";
import { SelectRange } from "./project/range";
import { ProjectHeader } from "./project/project";
import { SelectMetrics } from "./project/metric";
import { SelectFilters } from "./project/filter";
import { DimensionCard, DimensionDropdownCard, DimensionTabs, DimensionTabsCard, cardStyles } from "./dimensions";

const WorldMap = lazy(() => import("./worldmap").then((module) => ({ default: module.WorldMap })));

export type ProjectQuery = {
	project: ProjectResponse;
	metric: Metric;
	range: DateRange;
	filters: DimensionFilter[];
};

export const Project = () => {
	const [projectId, setProjectId] = useState<string | undefined>();
	const [filters, setFilters] = useState<DimensionFilter[]>([]);
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	useEffect(() => {
		if (typeof window === "undefined") return;
		setProjectId(window?.document.location.pathname.split("/").pop());
	}, []);

	const { project } = useProject(projectId);
	const { graph, stats } = useProjectData({ project, metric, rangeName: dateRange, filters });
	const { range } = resolveRange(dateRange);
	if (!project) return null;

	const query = { metric, range, filters, project };

	const toggleFilter = (filter: DimensionFilter) => {
		const index = filters.findIndex((f) => f.dimension === filter.dimension && f.filterType === filter.filterType);
		if (index === -1) {
			setFilters([...filters, filter]);
		} else {
			setFilters(filters.filter((_, i) => i !== index));
		}
	};

	const onSelectDimRow = (value: DimensionTableRow, dimension: Dimension) => {
		let filter: DimensionFilter = {
			dimension,
			filterType: "equal",
			value: value.dimensionValue,
		};

		if (dimension === "city") {
			// remove the first two characters from the dimension value
			// which are the country code
			filter = {
				dimension: "city",
				filterType: "equal",
				value: value.dimensionValue.slice(2),
			};
		}

		if (dimension === "mobile") {
			filter = {
				dimension: "mobile",
				filterType: value.dimensionValue === "true" ? "is_true" : "is_false",
			};
		}

		if (value.dimensionValue === "Unknown") {
			filter = {
				dimension,
				filterType: "is_null",
			};
		}

		toggleFilter(filter);
	};

	return (
		<div className={styles.project}>
			<Suspense fallback={null}>
				<div>
					<div className={styles.projectHeader}>
						<ProjectHeader project={project} stats={stats.data} />
						<SelectRange onSelect={(name: RangeName) => setDateRange(name)} range={dateRange} />
					</div>
					<SelectMetrics data={stats.data} metric={metric} setMetric={setMetric} className={styles.projectStats} />
					<SelectFilters value={filters} onChange={setFilters} />
				</div>
				<article className={cls(cardStyles, styles.graphCard)}>
					<LineGraph data={graph.data} title={metricNames[metric]} range={graph.range} />
				</article>
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
					<WorldMap data={data ?? []} metric={query.metric} />
				</Suspense>
			</div>
			<div className={styles.geoTable}>
				<DimensionTabs dimensions={["country", "city"]} query={query} onSelect={onSelect} />
			</div>
		</article>
	);
};
