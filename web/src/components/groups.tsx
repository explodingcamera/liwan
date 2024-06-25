import { fetchGroupGraph, fetchGroupStats, fetchGroups, useQuery } from "../api";
import { ranges } from "../api/ranges";
import { LineGraph, toDataPoints } from "./graph";
import styles from "./groups.module.css";

export const Groups = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["groups"],
		queryFn: fetchGroups,
	});

	return (
		<div>
			{data &&
				Object.entries(data).map(([key, value]) => {
					return <Group key={key} id={key} displayName={value.displayName} />;
				})}
		</div>
	);
};

const Group = ({ id, displayName }: { id: string; displayName: string }) => {
	const { data } = useQuery({
		queryKey: ["group_stats", id],
		queryFn: () => fetchGroupStats(id, { range: ranges.last7Days() }),
	});

	const dataRange = ranges.last7Days();
	const dataPoints = 7;

	const { data: data2 } = useQuery({
		queryKey: ["group_graph", id],
		queryFn: () => fetchGroupGraph(id, { range: ranges.last7Days(), metric: "views", dataPoints: 7 }),
	});

	const chartData = data2 ? toDataPoints(data2, dataRange) : [];
	console.log(data2);

	return (
		<>
			<article className={styles.stats}>
				<div>
					<h1>{displayName}</h1>
					{data && (
						<div className={styles.statsGrid}>
							<div>
								<h2>Total Views</h2>
								<h3>{data.totalViews}</h3>
							</div>
							<div>
								<h2>Total Sessions</h2>
								<h3>{data.totalSessions}</h3>
							</div>
							<div>
								<h2>Unique Visitors</h2>
								<h3>{data.uniqueVisitors}</h3>
							</div>
							<div>
								<h2>Avg Views Per Session</h2>
								<h3>{data.avgViewsPerSession / 1000}</h3>
							</div>
						</div>
					)}
				</div>
				<div>
					<button className="secondary outline" type="button">
						Details
					</button>
				</div>
			</article>
			{data2 && (
				<div className={styles.graph}>
					<LineGraph data={chartData} />
				</div>
			)}
		</>
	);
};
