import { Suspense } from "react";
import styles from "./projects.module.css";

import { useLocalStorage } from "@uidotdev/usehooks";
import { ChevronRightIcon } from "lucide-react";

import { type Metric, type ProjectResponse, api, metricNames, useMe, useProjectData, useQuery } from "../api";
import type { RangeName } from "../api/ranges";
import { cls, getUsername } from "../utils";
import { LineGraph } from "./graph";
import { SelectRange } from "./project/range";
import { SelectMetrics } from "./project/metric";
import { ProjectHeader } from "./project/project";

const signedIn = getUsername();

// Only load the role if no projects are available
const NoProjects = () => {
	const { role } = useMe();
	return (
		<div className={styles.info}>
			{role === "admin" ? (
				<h3>
					You do not have any projects yet.
					<br />
					<a href="/settings/projects">Create a new project</a>
					&nbsp;to get started.
				</h3>
			) : (
				<h3>
					You do not have any projects yet.
					<br />
					Contact an admin to create a new project.
				</h3>
			)}
		</div>
	);
};

export const Projects = () => {
	const { data, isLoading, isError } = useQuery({
		queryKey: ["projects"],
		queryFn: () => api["/api/dashboard/projects"].get().json(),
	});

	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	const projects = data?.projects || [];

	if (isLoading) return null;
	if (isError)
		return (
			<div className={styles.info}>
				<h1>Failed to load data</h1>
			</div>
		);

	if (projects.length === 0 && signedIn) return <NoProjects />;
	if (projects.length === 0 && !signedIn)
		return (
			<div className={styles.info}>
				<h3>
					There are no public projects available.
					<br />
					<a href="/login">Sign in</a> to view all projects.
				</h3>
			</div>
		);

	return (
		<div className={styles.projects}>
			<div className={styles.header}>
				<h1>Dashboard</h1>
				<SelectRange onSelect={(name: RangeName) => setDateRange(name)} range={dateRange} />
			</div>

			<Suspense>
				{projects.map((project) => (
					<Project key={project.id} project={project} metric={metric} setMetric={setMetric} rangeName={dateRange} />
				))}
			</Suspense>
		</div>
	);
};

const Project = ({
	project,
	metric,
	setMetric,
	rangeName,
}: { project: ProjectResponse; metric: Metric; setMetric: (value: Metric) => void; rangeName: RangeName }) => {
	const { stats, graph, isLoading, isError } = useProjectData({ project, metric, rangeName });

	return (
		<article className={styles.project} data-loading={isLoading || isError}>
			{isError && <h1 className={styles.error}>Failed to load data</h1>}
			<div className={styles.projectHeader}>
				<div className={styles.stats}>
					<ProjectHeader project={project} stats={stats.data} />
					<div>
						<SelectMetrics data={stats.data} metric={metric} setMetric={setMetric} />
					</div>
				</div>
				<a href={`/p/${project.id}`} aria-label="View project details">
					<ChevronRightIcon size={25} strokeWidth={4} color="var(--pico-h1-color)" />
				</a>
			</div>
			<div className={styles.graph}>
				<LineGraph title={metricNames[metric]} data={graph.data} range={graph.range} />
			</div>
		</article>
	);
};
