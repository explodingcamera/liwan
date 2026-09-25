import styles from "./projects.module.css";

import { useEffect, useState } from "react";
import { Accordion } from "@base-ui/react/accordion";
import { ChevronDownIcon } from "lucide-react";

import { api, useQuery } from "@/api";
import { DateRange } from "@/api/ranges";
import { LineGraph } from "@/components/dashboard/project/graph";
import { SelectMetrics } from "@/components/dashboard/project/metric";
import { ProjectHeader } from "@/components/dashboard/project/project-header";
import { SelectRange } from "@/components/dashboard/project/range";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Metric, ProjectResponse } from "@/constants";
import { metricNames, metrics } from "@/constants";
import { useMe, useProjectGraph, useProjectStats } from "@/hooks/api";
import { useMetric, useRange } from "@/hooks/persist";
import { getUsername } from "@/utils";

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
		placeholderData: (previous) => previous,
		queryFn: () => api["/api/dashboard/projects"].get().json(),
	});

	const { metric, setMetric } = useMetric();
	const { range, setRange } = useRange();
	const [hiddenProjects, setHiddenProjects] = useState<string[]>([]);
	const visibleMetrics = metrics.filter((metric) =>
		data?.projects.some((project) => !project.hiddenMetrics.includes(metric)),
	);
	const activeMetric = visibleMetrics.includes(metric) ? metric : visibleMetrics[0];

	useEffect(() => {
		if (activeMetric && activeMetric !== metric) setMetric(activeMetric);
	}, [activeMetric, metric, setMetric]);

	useEffect(() => {
		if (range.variant === "allTime") setRange(new DateRange(range.value));
	}, [range, setRange]);

	if (isError)
		return (
			<div className={styles.info}>
				<h1>Failed to load dashboard</h1>
			</div>
		);

	if (data?.projects.length === 0 && signedIn) return <NoProjects />;
	if (data?.projects.length === 0 && !signedIn)
		return (
			<div className={styles.info}>
				<h3>
					There are no public projects available.
					<br />
					<a href="/login">Login</a> to view all projects.
				</h3>
			</div>
		);

	return (
		<div className={styles.projects}>
			<div className={styles.header}>
				<SelectRange onSelect={setRange} range={range} />
			</div>

			{isLoading && <LoadingSpinner />}
			{data && (
				<Accordion.Root
					className={styles.projectList}
					multiple
					onValueChange={(value) =>
						setHiddenProjects(data?.projects.map((p) => p.id).filter((id) => !value.includes(id)) ?? [])
					}
					value={data?.projects.map((p) => p.id).filter((id) => !hiddenProjects.includes(id))}
				>
					{data?.projects.map((project) => (
						<Accordion.Item key={project.id} value={project.id}>
							<Project project={project} metric={activeMetric ?? "views"} setMetric={setMetric} range={range} />
						</Accordion.Item>
					))}
				</Accordion.Root>
			)}
		</div>
	);
};

const Project = ({
	project,
	metric,
	setMetric,
	range,
}: {
	project: ProjectResponse;
	metric: Metric;
	setMetric: (value: Metric) => void;
	range: DateRange;
}) => {
	const visibleMetrics = metrics.filter((metric) => !project.hiddenMetrics.includes(metric));
	const reportMetric = visibleMetrics.includes(metric) ? metric : visibleMetrics[0];
	const {
		graph,
		displayMetric,
		displayRange,
		isLoading: graphLoading,
		isUpdating: graphUpdating,
	} = useProjectGraph({
		projectId: project.id,
		metric: reportMetric ?? "views",
		range,
		enabled: Boolean(reportMetric),
	});

	const {
		stats,
		isLoading: statsLoading,
		isUpdating: statsUpdating,
	} = useProjectStats({
		projectId: project.id,
		range,
		enabled: Boolean(reportMetric),
	});

	return (
		<article className={styles.project}>
			<div className={styles.projectHeader}>
				<div className={styles.projectTitle}>
					<ProjectHeader project={project} stats={stats} />
					<Accordion.Trigger className={styles.AccordionTrigger} aria-label={`Toggle ${project.displayName} details`}>
						<ChevronDownIcon size={22} strokeWidth={2} color="var(--text-strong)" />
					</Accordion.Trigger>
				</div>
				<SelectMetrics
					data={stats}
					metric={reportMetric ?? "views"}
					metrics={visibleMetrics}
					setMetric={setMetric}
					isLoading={statsLoading || statsUpdating}
				/>
			</div>
			{reportMetric && (
				<Accordion.Panel className={styles.AccordionContent}>
					<div className={styles.graph}>
						<LineGraph
							data={graph}
							title={metricNames[displayMetric]}
							metric={displayMetric}
							isLoading={graphLoading}
							isUpdating={graphUpdating}
							range={displayRange}
						/>
					</div>
				</Accordion.Panel>
			)}
		</article>
	);
};
