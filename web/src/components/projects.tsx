import { useMemo, useRef } from "react";
import { api, metricNames, useMe, useQuery, type Metric } from "../api";
import { rangeNames, resolveRange, type RangeName } from "../api/ranges";
import { LineGraph, toDataPoints } from "./graph";
import styles from "./projects.module.css";
import CountUp from "react-countup";
import { CircleIcon, LockIcon } from "lucide-react";
import { useLocalStorage } from "@uidotdev/usehooks";
import { getUsername } from "../api/utils";

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

	const detailsRef = useRef<HTMLDetailsElement>(null);
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	const onSelect = (name: RangeName) => () => {
		if (detailsRef.current) detailsRef.current.open = false;
		setDateRange(name);
	};
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
				<details ref={detailsRef} className="dropdown">
					<summary>{rangeNames[dateRange]}</summary>
					<ul>
						{Object.entries(rangeNames).map(([key, value]) => (
							<li key={key}>
								{/* biome-ignore lint/a11y/useValidAnchor: <explanation> */}
								<a className={key === dateRange ? styles.selected : ""} onClick={onSelect(key as RangeName)}>
									{value}
								</a>
							</li>
						))}
					</ul>
				</details>
			</div>

			{projects.map((project) => {
				return (
					<Project
						key={project.id}
						rangeName={dateRange}
						id={project.id}
						displayName={project.displayName}
						isPublic={project.public}
						metric={metric}
						setMetric={setMetric}
					/>
				);
			})}
		</div>
	);
};

const Project = ({
	id,
	displayName,
	rangeName,
	metric,
	setMetric,
	isPublic,
}: {
	id: string;
	displayName: string;
	rangeName: RangeName;
	metric: Metric;
	setMetric: (value: Metric) => void;
	isPublic: boolean;
}) => {
	const { range, graphRange, dataPoints } = useMemo(() => resolveRange(rangeName), [rangeName]);

	let refetchInterval = undefined;
	let staleTime = 1000 * 60 * 10;
	if (rangeName === "today") {
		refetchInterval = 1000 * 60;
		staleTime = 1000 * 60;
	}

	const {
		data: stats,
		isError: isErrorStats,
		isLoading: isLoadingStats,
	} = useQuery({
		refetchInterval,
		staleTime,
		queryKey: ["project_stats", id, range],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/stats"].post({ json: { range }, params: { project_id: id } }).json(),
		placeholderData: (prev) => prev,
	});

	const json = { range, metric, dataPoints };
	const {
		data: graph,
		isError: isErrorGraph,
		isLoading: isLoadingGraph,
	} = useQuery({
		refetchInterval,
		staleTime,
		queryKey: ["project_graph", id, range, graphRange, metric, dataPoints],
		queryFn: () => api["/api/dashboard/project/{project_id}/graph"].post({ json, params: { project_id: id } }).json(),
		placeholderData: (prev) => prev,
	});

	const chartData = graph?.data ? toDataPoints(graph.data, range, metric) : [];

	return (
		<div className={styles.project} data-loading={isLoadingStats || isLoadingGraph || isErrorStats || isErrorGraph}>
			{(isErrorStats || isErrorGraph) && <h1 className={styles.error}>Failed to load data</h1>}
			<div className={styles.stats}>
				<h1>
					<span>
						{isPublic ? null : (
							<>
								<LockIcon size={16} />
								&nbsp;
							</>
						)}
						{displayName}&nbsp;
					</span>
					<span className={styles.online}>
						<CircleIcon fill="#22c55e" color="#22c55e" size={10} />
						<CountUp preserveValue duration={1} end={0} /> Current Visitors
					</span>
				</h1>
				<div>
					<button type="button" data-active={metric === "views"} onClick={() => setMetric("views")}>
						<h2>Total Views</h2>
						<h3>
							<CountUp preserveValue duration={1} end={stats?.totalViews || 0} />
						</h3>
					</button>
					<button type="button" data-active={metric === "sessions"} onClick={() => setMetric("sessions")}>
						<h2>Total Sessions</h2>
						<h3>
							<CountUp preserveValue duration={1} end={stats?.totalSessions || 0} />
						</h3>
					</button>
					<button type="button" data-active={metric === "unique_visitors"} onClick={() => setMetric("unique_visitors")}>
						<h2>Unique Visitors</h2>
						<h3>
							<CountUp preserveValue duration={1} end={stats?.uniqueVisitors || 0} />
						</h3>
					</button>
					<button
						type="button"
						data-active={metric === "avg_views_per_session"}
						onClick={() => setMetric("avg_views_per_session")}
					>
						<h2>Avg Views Per Session</h2>
						<h3>
							<CountUp preserveValue decimals={1} duration={1} end={(stats?.avgViewsPerSession || 0) / 1000} />
						</h3>
					</button>
				</div>
			</div>

			<div className={styles.graph}>
				<LineGraph title={metricNames[metric]} data={chartData || []} range={graphRange} />
			</div>
		</div>
	);
};
