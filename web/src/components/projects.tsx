import { Suspense, useMemo } from "react";
import styles from "./projects.module.css";

import { useLocalStorage } from "@uidotdev/usehooks";
import { ChevronDownIcon } from "lucide-react";

import { type Metric, type ProjectResponse, api, metricNames, useMe, useProjectData, useQuery } from "../api";
import { DateRange } from "../api/ranges";
import { getUsername } from "../utils";
import { LineGraph } from "./graph";
import { SelectRange } from "./project/range";
import { SelectMetrics } from "./project/metric";
import { ProjectHeader } from "./project/project";
import * as Accordion from "@radix-ui/react-accordion";

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

	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");
	const [hiddenProjects, setHiddenProjects] = useLocalStorage<string[]>("hiddenProjects", []);
	const [rangeString, setRangeString] = useLocalStorage<string>("date-range", "last7Days");
	const range = useMemo(() => DateRange.deserialize(rangeString), [rangeString]);

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
				<SelectRange onSelect={(range) => setRangeString(range.serialize())} range={range} />
			</div>

			<Suspense>
				<Accordion.Root
					className="AccordionRoot"
					type="multiple"
					onValueChange={(visible) =>
						setHiddenProjects(projects.map((p) => p.id).filter((id) => !visible.includes(id)))
					}
					value={projects.map((p) => p.id).filter((id) => !hiddenProjects.includes(id))}
				>
					{projects.map((project) => (
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
}: { project: ProjectResponse; metric: Metric; setMetric: (value: Metric) => void; range: DateRange }) => {
	const { stats, graph, isLoading, isError } = useProjectData({ project, metric, range });

	return (
		<article className={styles.project} data-loading={isLoading || isError} data-error={isError}>
			<div className={styles.projectHeader}>
				<div className={styles.projectTitle}>
					<ProjectHeader project={project} stats={stats.data} />
					<Accordion.Trigger className={styles.AccordionTrigger} aria-label="Toggle details">
						<ChevronDownIcon size={35} strokeWidth={2} color="var(--pico-h1-color)" />
					</Accordion.Trigger>
				</div>
				<SelectMetrics data={stats.data} metric={metric} setMetric={setMetric} />
			</div>
			<Accordion.AccordionContent className={styles.AccordionContent}>
				<div className={styles.graph}>
					<LineGraph title={metricNames[metric]} metric={metric} data={graph.data} range={graph.range} />
				</div>
			</Accordion.AccordionContent>
		</article>
	);
};
