import styles from "./project.module.css";
import _map from "./worldmap.module.css";

import { useLocalStorage } from "@uidotdev/usehooks";
import { Suspense, lazy, useEffect, useState } from "react";

import { metricNames, useDimension, useProject, useProjectData } from "../api";
import type { DateRange, Metric, ProjectResponse } from "../api";
import { type RangeName, resolveRange } from "../api/ranges";

import { cls } from "../utils";
import { DimensionCard, DimensionTabs, DimensionTabsCard, cardStyles } from "./dimensions";
import { SelectRange } from "./project/range";
import { ProjectHeader } from "./project/project";
import { SelectMetrics } from "./project/metric";
import { LineGraph } from "./graph";

const WorldMap = lazy(() => import("./worldmap").then((module) => ({ default: module.WorldMap })));

export const Project = () => {
	const [projectId, setProjectId] = useState<string | undefined>();
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	useEffect(() => {
		if (typeof window === "undefined") return;
		setProjectId(window?.document.location.pathname.split("/").pop());
	}, []);

	const { project, isLoading, error } = useProject(projectId);
	const { graph, stats } = useProjectData({ project, metric, rangeName: dateRange });
	const { range } = resolveRange(dateRange);
	if (!project) return null;

	return (
		<div className={styles.project}>
			<Suspense fallback={null}>
				<div>
					<div className={styles.projectHeader}>
						<ProjectHeader project={project} stats={stats.data} />
						<SelectRange onSelect={(name: RangeName) => setDateRange(name)} range={dateRange} />
					</div>
					<SelectMetrics data={stats.data} metric={metric} setMetric={setMetric} />
				</div>
				<article className={cls(cardStyles, styles.graphCard)}>
					<LineGraph data={graph.data} title={metricNames[metric]} range={graph.range} />
				</article>
				<div className={styles.tables}>
					<DimensionTabsCard project={project} dimensions={["url", "fqdn"]} metric={metric} range={range} />
					<DimensionCard project={project} dimension={"referrer"} metric={metric} range={range} />
					<GeoCard project={project} metric={metric} range={range} />
					<DimensionTabsCard project={project} dimensions={["platform", "browser"]} metric={metric} range={range} />
					<DimensionCard project={project} dimension={"mobile"} metric={metric} range={range} />
				</div>
			</Suspense>
		</div>
	);
};

const GeoCard = ({ project, metric, range }: { project: ProjectResponse; metric: Metric; range: DateRange }) => {
	const { data } = useDimension({
		dimension: "country",
		metric,
		project,
		range,
	});

	return (
		<article className={cls(cardStyles, styles.geoCard)} data-full-width="true">
			<div className={styles.geoMap}>
				<Suspense fallback={null}>
					<WorldMap data={data ?? []} metric={metric} />
				</Suspense>
			</div>
			<div className={styles.geoTable}>
				<DimensionTabs dimensions={["country", "city"]} project={project} metric={metric} range={range} />
			</div>
		</article>
	);
};
