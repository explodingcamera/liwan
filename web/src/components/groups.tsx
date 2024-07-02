import { useMemo, useRef } from "react";
import { api, metricNames, useQuery, type Metric } from "../api";
import { rangeNames, resolveRange, type RangeName } from "../api/ranges";
import { LineGraph, toDataPoints } from "./graph";
import styles from "./groups.module.css";
import CountUp from "react-countup";
import { CircleIcon, LockIcon } from "lucide-react";
import { useLocalStorage } from "@uidotdev/usehooks";

export const Groups = () => {
	const { data } = useQuery({
		queryKey: ["groups"],
		queryFn: () => api["/api/dashboard/groups"].get().json(),
	});

	const detailsRef = useRef<HTMLDetailsElement>(null);
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	const onSelect = (name: RangeName) => () => {
		if (detailsRef.current) detailsRef.current.open = false;
		setDateRange(name);
	};

	return (
		<div>
			<div className={styles.settings}>
				<details ref={detailsRef} className="dropdown">
					<summary>{rangeNames[dateRange]}</summary>
					<ul>
						{Object.entries(rangeNames).map(([key, value]) => (
							<li key={key}>
								{/* biome-ignore lint/a11y/useValidAnchor: <explanation> */}
								<a className={key === dateRange ? "selected" : ""} onClick={onSelect(key as RangeName)}>
									{value}
								</a>
							</li>
						))}
					</ul>
				</details>
			</div>

			{data &&
				Object.entries(data.groups).map(([key, value]) => {
					return (
						<Group
							key={key}
							rangeName={dateRange}
							id={key}
							displayName={value.displayName}
							isPublic={value.public}
							metric={metric}
							setMetric={setMetric}
						/>
					);
				})}
		</div>
	);
};

const Group = ({
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

	const { data: stats } = useQuery({
		queryKey: ["group_stats", id, range],
		queryFn: () =>
			api["/api/dashboard/group/{group_id}/stats"].post({ json: { range }, params: { group_id: id } }).json(),
		placeholderData: (prev) => prev,
	});

	const json = { range, metric, dataPoints };
	const { data: graph } = useQuery({
		queryKey: ["group_graph", id, range, graphRange, metric, dataPoints],
		queryFn: () => api["/api/dashboard/group/{group_id}/graph"].post({ json, params: { group_id: id } }).json(),
		placeholderData: (prev) => prev,
	});

	const chartData = graph ? toDataPoints(graph.data, range, metric) : [];

	return (
		<>
			<div className={styles.stats}>
				<div>
					<h1 className={styles.header}>
						{isPublic ? null : (
							<>
								<LockIcon size={16} />
								&nbsp;
							</>
						)}
						{displayName}&nbsp;
						<span>
							<CircleIcon fill="#22c55e" color="#22c55e" size={10} />
							<CountUp preserveValue duration={1} end={0} /> Current Visitors
						</span>
					</h1>
					{stats && (
						<div className={styles.statsGrid}>
							<button type="button" data-active={metric === "views"} onClick={() => setMetric("views")}>
								<h2>Total Views</h2>
								<h3>
									<CountUp preserveValue duration={1} end={stats.totalViews} />
								</h3>
							</button>
							<button type="button" data-active={metric === "sessions"} onClick={() => setMetric("sessions")}>
								<h2>Total Sessions</h2>
								<h3>
									<CountUp preserveValue duration={1} end={stats.totalSessions} />
								</h3>
							</button>
							<button
								type="button"
								data-active={metric === "unique_visitors"}
								onClick={() => setMetric("unique_visitors")}
							>
								<h2>Unique Visitors</h2>
								<h3>
									<CountUp preserveValue duration={1} end={stats.uniqueVisitors} />
								</h3>
							</button>
							<button
								type="button"
								data-active={metric === "avg_views_per_session"}
								onClick={() => setMetric("avg_views_per_session")}
							>
								<h2>Avg Views Per Session</h2>
								<h3>
									<CountUp preserveValue decimals={1} duration={1} end={stats.avgViewsPerSession / 1000} />
								</h3>
							</button>
						</div>
					)}
				</div>
				{/* <div>
					<button className="secondary outline" type="button">
						Details
					</button>
				</div> */}
			</div>
			{stats && (
				<div className={styles.graph}>
					<LineGraph title={metricNames[metric]} data={chartData} range={graphRange} />
				</div>
			)}
		</>
	);
};
