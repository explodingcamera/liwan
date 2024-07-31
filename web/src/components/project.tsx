import { useEffect, useMemo, useState } from "react";
import styles from "./project.module.css";

import { api, useQuery, type DateRange, type Dimension, type Metric, type ProjectResponse } from "../api";
import { ProjectOverview } from "./projects";
import { useLocalStorage } from "@uidotdev/usehooks";
import { resolveRange, type RangeName } from "../api/ranges";
const server = typeof window === "undefined";

export const Project = () => {
	const [projectId, setProjectId] = useState<string | null>(null);
	const [dateRange, setDateRange] = useLocalStorage<RangeName>("date-range", "last7Days");
	const [metric, setMetric] = useLocalStorage<Metric>("metric", "views");

	useEffect(() => {
		if (server) return;
		setProjectId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);

	const { data, isLoading, error } = useQuery({
		enabled: projectId !== null,
		queryKey: ["project", projectId],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}"].get({ params: { project_id: projectId as string } }).json(),
	});

	if (!data) return null;

	return (
		<div className={styles.project}>
			<h1>{data.displayName}</h1>
			<Entities entities={data.entities} />
			<ProjectOverview project={data} metric={metric} setMetric={setMetric} rangeName={dateRange} />
			<Test project={data} dimension={"browser"} metric={metric} range={resolveRange(dateRange).range} />
		</div>
	);
};

const Entities = ({ entities }: { entities: { id: string; displayName: string }[] }) => {
	return (
		<div className={styles.entities}>
			{entities.map((entity) => (
				<div key={entity.id} className={styles.entity}>
					<h3>{entity.displayName}</h3>
				</div>
			))}
		</div>
	);
};

const Test = ({
	project,
	dimension,
	metric,
	range,
}: { project: ProjectResponse; dimension: Dimension; metric: Metric; range: DateRange }) => {
	const { data } = useQuery({
		queryKey: ["dimension", project.id, dimension, metric, range],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/dimension"]
				.post({
					params: { project_id: project.id },
					json: {
						dimension,
						metric,
						range,
					},
				})
				.json(),
	});

	return (
		<div>
			{/* {data?.data?.map((d) => (
				<div key={d.dimensionValue}>
					<h2>{d.dispayName ?? d.dimensionValue}</h2>
					<p>{d.value}</p>
				</div>
			))} */}
			<table>
				<tbody>
					{data?.data?.map((d) => (
						<tr key={d.dimensionValue}>
							<td>{d.dispayName ?? d.dimensionValue}</td>
							<td>
								{d.value} ({d.icon})
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
};
