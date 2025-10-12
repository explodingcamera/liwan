import { Suspense, useState } from "react";
import styles from "./projects.module.css";

import { ChevronDownIcon } from "lucide-react";

import * as Accordion from "@radix-ui/react-accordion";
import {
	type Metric,
	type ProjectResponse,
	api,
	metricNames,
	useMe,
	useProjectGraph,
	useProjectStats,
	useQuery,
} from "../api";
import type { DateRange } from "../api/ranges";
import { useMetric, useRange } from "../hooks/persist";
import { getUsername } from "../utils";
import { LineGraph } from "./graph";
import { SelectMetrics } from "./project/metric";
import { ProjectHeader } from "./project/project";
import { SelectRange } from "./project/range";

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

	if (isLoading) return null;
	if (isError)
		return (
			<div className={styles.info}>
				<h1>Failed to load dashboard</h1>
			</div>
		);

	if (data?.projects.length === 0 && !isLoading && signedIn) return <NoProjects />;
	if (data?.projects.length === 0 && !signedIn)
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
				<SelectRange onSelect={setRange} range={range} />
			</div>

			<Suspense>
				<Accordion.Root
					className="AccordionRoot"
					type="multiple"
					onValueChange={(value) =>
						setHiddenProjects(data?.projects.map((p) => p.id).filter((id) => !value.includes(id)) ?? [])
					}
					value={data?.projects.map((p) => p.id).filter((id) => !hiddenProjects.includes(id))}
				>
					{data?.projects.map((project) => (
						<Accordion.Item key={project.id} value={project.id}>
							<Project project={project} metric={metric} setMetric={setMetric} range={range} />
						</Accordion.Item>
					))}
				</Accordion.Root>
			</Suspense>
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
	const {
		graph,
		isError: graphError,
		isLoading: graphLoading,
	} = useProjectGraph({ projectId: project.id, metric, range });

	const {
		stats,
		isError: statsError,
		isLoading: statsLoading,
	} = useProjectStats({ projectId: project.id, metric, range });

	const isLoading = graphLoading || statsLoading;
	const isError = graphError || statsError;

	return (
		<article className={styles.project} data-loading={isLoading || isError} data-error={isError}>
			<div className={styles.projectHeader}>
				<div className={styles.projectTitle}>
					<ProjectHeader project={project} stats={stats} />
					<Accordion.Trigger className={styles.AccordionTrigger} aria-label="Toggle details">
						<ChevronDownIcon size={35} strokeWidth={2} color="var(--pico-h1-color)" />
					</Accordion.Trigger>
				</div>
				<SelectMetrics data={stats} metric={metric} setMetric={setMetric} />
			</div>
			<Accordion.AccordionContent className={styles.AccordionContent}>
				<div className={styles.graph}>
					<LineGraph title={metricNames[metric]} metric={metric} data={graph ?? []} range={range} />
				</div>
			</Accordion.AccordionContent>
		</article>
	);
};
