import styles from "./project.module.css";
import "./worldmap.module.css";

import { lazy, Suspense, useEffect, useState } from "react";
import { LockIcon } from "lucide-react";
import { useLocalStorage } from "@uidotdev/usehooks";

import { resolveRange, type RangeName } from "../api/ranges";
import { api, useDimension, useQuery } from "../api";
import type { DateRange, Metric, ProjectResponse, StatsResponse } from "../api";

import { LiveVisitorCount, ProjectOverview, SelectRange } from "./projects";
import { cardStyles, DimensionCard, DimensionTabs, DimensionTabsCard } from "./dimensions";

const WorldMap = lazy(() => import("./worldmap").then((module) => ({ default: module.WorldMap })));

export const Project = () => {
	const [projectId, setProjectId] = useState<string | null>(null);
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	useEffect(() => {
		if (typeof window === "undefined") return;
		setProjectId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);

	const { data, isLoading, error } = useQuery({
		enabled: projectId !== null,
		queryKey: ["project", projectId],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}"].get({ params: { project_id: projectId as string } }).json(),
	});

	if (!data) return null;
	const range = resolveRange(dateRange).range;

	return (
		<div className={styles.project}>
			<ProjectOverview
				project={data}
				metric={metric}
				setMetric={setMetric}
				rangeName={dateRange}
				graphClassName={styles.graph}
				renderHeader={(props) => <ProjectHeader {...props} range={dateRange} setRange={setDateRange} />}
			/>
			<div className={styles.tables}>
				<DimensionTabsCard project={data} dimensions={["url", "fqdn"]} metric={metric} range={range} />
				<DimensionCard project={data} dimension={"referrer"} metric={metric} range={range} />
				<GeoCard project={data} metric={metric} range={range} />
				<DimensionTabsCard project={data} dimensions={["platform", "browser"]} metric={metric} range={range} />
				<DimensionCard project={data} dimension={"mobile"} metric={metric} range={range} />
			</div>
		</div>
	);
};

const ProjectHeader = ({
	project,
	stats,
	range,
	setRange,
	className,
}: {
	stats?: StatsResponse;
	project: ProjectResponse;
	range: RangeName;
	setRange: (range: RangeName) => void;
	className?: string;
}) => {
	return (
		<div className={styles.projectHeader}>
			<h1 className={className}>
				<span>
					{project.public ? null : (
						<>
							<LockIcon size={16} />
							&nbsp;
						</>
					)}
					<a href={`/p/${project.id}`}>{project.displayName}</a>&nbsp;
				</span>
				<LiveVisitorCount count={stats?.currentVisitors || 0} />
			</h1>
			<SelectRange range={range} onSelect={setRange} />
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
		<div className={`${cardStyles} ${styles.geoCard}`} data-full-width="true">
			<div className={styles.geoMap}>
				<Suspense fallback={null}>
					<WorldMap data={data ?? []} metric={metric} />
				</Suspense>
			</div>
			<div className={styles.geoTable}>
				<DimensionTabs dimensions={["country", "city"]} project={project} metric={metric} range={range} />
			</div>
		</div>
	);
};
