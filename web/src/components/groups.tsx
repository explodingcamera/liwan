import { fetchGroupGraph, fetchGroupStats, fetchGroups, useQuery } from "../api";
import { ranges } from "../api/ranges";
import { LineGraph, toDataPoints } from "./graph";

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
			<h4>{displayName}</h4>
			{data && (
				<>
					<article>
						<p>Total Views: {data.totalViews}</p>
						<p>Total Sessions: {data.totalSessions}</p>
						<p>Unique Visitors: {data.uniqueVisitors}</p>
						<p>Avg Views Per Session: {data.avgViewsPerSession / 1000}</p>
					</article>
					<div style={{ height: "20rem" }}>
						<LineGraph data={chartData} />
					</div>
				</>
			)}
		</>
	);
};
