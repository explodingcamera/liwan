import { useMemo, useRef, useState } from "react";
import { fetchGroupGraph, fetchGroupStats, fetchGroups, useQuery } from "../api";
import { rangeNames, resolveRange, type RangeName } from "../api/ranges";
import { LineGraph, toDataPoints } from "./graph";
import styles from "./groups.module.css";
import CountUp from "react-countup";

export const Groups = () => {
	const { data } = useQuery({
		queryKey: ["groups"],
		queryFn: fetchGroups,
	});

	const detailsRef = useRef<HTMLDetailsElement>(null);
	const [dateRange, setDateRange] = useState<RangeName>("last7Days");

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
				Object.entries(data).map(([key, value]) => {
					return <Group key={key} rangeName={dateRange} id={key} displayName={value.displayName} />;
				})}
		</div>
	);
};

const Group = ({ id, displayName, rangeName }: { id: string; displayName: string; rangeName: RangeName }) => {
	const { range, graphRange, dataPoints } = useMemo(() => resolveRange(rangeName), [rangeName]);

	const { data } = useQuery({
		queryKey: ["group_stats", id, range],
		queryFn: () => fetchGroupStats(id, { range }),
		placeholderData: (prev) => prev,
	});

	const { data: data2 } = useQuery({
		queryKey: ["group_graph", id, range, graphRange, dataPoints],
		queryFn: () => fetchGroupGraph(id, { range, metric: "views", dataPoints }),
		placeholderData: (prev) => prev,
	});

	const chartData = data2 ? toDataPoints(data2, range) : [];

	return (
		<>
			<article className={styles.stats}>
				<div>
					<h1>{displayName}</h1>
					{data && (
						<div className={styles.statsGrid}>
							<div>
								<h2>Total Views</h2>
								<h3>
									<CountUp preserveValue duration={1} end={data.totalViews} />
								</h3>
							</div>
							<div>
								<h2>Total Sessions</h2>
								<h3>
									<CountUp preserveValue duration={1} end={data.totalSessions} />
								</h3>
							</div>
							<div>
								<h2>Unique Visitors</h2>
								<h3>
									<CountUp preserveValue duration={1} end={data.uniqueVisitors} />
								</h3>
							</div>
							<div>
								<h2>Avg Views Per Session</h2>
								<h3>
									<CountUp preserveValue decimals={1} duration={1} end={data.avgViewsPerSession / 1000} />
								</h3>
							</div>
						</div>
					)}
				</div>
				{/* <div>
					<button className="secondary outline" type="button">
						Details
					</button>
				</div> */}
			</article>
			{data2 && (
				<div className={styles.graph}>
					<LineGraph title="Views" data={chartData} range={graphRange} />
				</div>
			)}
		</>
	);
};
