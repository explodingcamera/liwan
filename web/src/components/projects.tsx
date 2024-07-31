import { useMemo, useRef } from "react";
import styles from "./projects.module.css";

import CountUp from "react-countup";
import { useLocalStorage } from "@uidotdev/usehooks";
import { ChevronRightIcon, CircleIcon, LockIcon, TrendingDownIcon, TrendingUpIcon } from "lucide-react";

import { getUsername } from "../api/utils";
import { LineGraph, toDataPoints } from "./graph";
import { rangeNames, resolveRange, type RangeName } from "../api/ranges";
import { api, metricNames, useMe, useQuery, type Metric, type ProjectResponse } from "../api";

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
								{/* biome-ignore lint/a11y/useValidAnchor: this is fine */}
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
					<ProjectOverview
						key={project.id}
						project={project}
						metric={metric}
						setMetric={setMetric}
						rangeName={dateRange}
					/>
				);
			})}
		</div>
	);
};

export const ProjectOverview = ({
	project,
	metric,
	setMetric,
	rangeName,
}: {
	project: ProjectResponse;
	metric: Metric;
	setMetric: (value: Metric) => void;
	rangeName: RangeName;
}) => {
	const { displayName, id, public: isPublic } = project;
	const { range, graphRange, dataPoints } = useMemo(() => resolveRange(rangeName), [rangeName]);

	let refetchInterval = undefined;
	let staleTime = 1000 * 60 * 10;
	if (rangeName === "today" || rangeName.startsWith("last")) {
		refetchInterval = 1000 * 60;
		staleTime = 0;
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
			<div className={styles.statsContainer}>
				<div className={styles.stats}>
					<h1>
						<span>
							{isPublic ? null : (
								<>
									<LockIcon size={16} />
									&nbsp;
								</>
							)}
							<a href={`/p/${id}`}>{displayName}</a>&nbsp;
						</span>
						<span className={styles.online}>
							<CircleIcon fill="#22c55e" color="#22c55e" size={10} />
							<CircleIcon fill="#22c55e" color="#22c55e" size={10} className={styles.pulse} />
							<CountUp preserveValue duration={1} end={stats?.currentVisitors || 0} />{" "}
							{stats?.currentVisitors === 1 ? "Current Visitor" : "Current Visitors"}
						</span>
					</h1>
					<div>
						<Stat
							title="Total Views"
							value={stats?.stats.totalViews}
							prevValue={stats?.statsPrev.totalViews}
							onSelect={() => setMetric("views")}
							selected={metric === "views"}
						/>

						<Stat
							title="Total Sessions"
							value={stats?.stats.totalSessions}
							prevValue={stats?.statsPrev.totalSessions}
							onSelect={() => setMetric("sessions")}
							selected={metric === "sessions"}
						/>
						<Stat
							title="Unique Visitors"
							value={stats?.stats.uniqueVisitors}
							prevValue={stats?.statsPrev.uniqueVisitors}
							onSelect={() => setMetric("unique_visitors")}
							selected={metric === "unique_visitors"}
						/>
						<Stat
							title="Avg Views Per Session"
							value={(stats?.stats.avgViewsPerSession ?? 0) / 1000}
							prevValue={(stats?.statsPrev.avgViewsPerSession ?? 0) / 1000}
							decimals={1}
							onSelect={() => setMetric("avg_views_per_session")}
							selected={metric === "avg_views_per_session"}
						/>
					</div>
				</div>
				<a href={`/p/${id}`}>
					<ChevronRightIcon size={25} strokeWidth={4} color="var(--pico-h1-color)" />
				</a>
			</div>

			<div className={styles.graph}>
				<LineGraph title={metricNames[metric]} data={chartData || []} range={graphRange} />
			</div>
		</div>
	);
};

const formatPercent = (value: number) => {
	if (value === -1) return "∞";
	return value.toFixed(1).replace(/\.0$/, "") || "0";
};

export const Stat = ({
	title,
	value = 0,
	prevValue = 0,
	decimals = 0,
	onSelect,
	selected,
}: {
	title: string;
	value?: number;
	prevValue?: number;
	decimals?: number;
	onSelect: () => void;
	selected: boolean;
}) => {
	const change = value - prevValue;
	const changePercent = prevValue ? (change / prevValue) * 100 : value ? -1 : 0;
	const color = change > 0 ? "#22c55e" : change < 0 ? "red" : "gray";
	const icon = change > 0 ? <TrendingUpIcon size={14} /> : change < 0 ? <TrendingDownIcon size={14} /> : "—";

	return (
		<button type="button" onClick={onSelect} data-active={selected} className={styles.stat}>
			<h2>{title}</h2>
			<h3>
				<CountUp preserveValue decimals={decimals} duration={1} end={value} />
				<span style={{ color }} className={styles.change}>
					{icon} {formatPercent(changePercent)}%
				</span>
			</h3>
		</button>
	);
};
