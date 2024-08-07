import { useEffect, useMemo, useState } from "react";
import styles from "./project.module.css";

import {
	api,
	dimensionNames,
	metricNames,
	useQuery,
	type DateRange,
	type Dimension,
	type Metric,
	type ProjectResponse,
} from "../api";
import { ProjectOverview } from "./projects";
import { useLocalStorage } from "@uidotdev/usehooks";
import { resolveRange, type RangeName } from "../api/ranges";
import { BrowserIcon, OSIcon } from "./icons";
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
			{/* <Entities entities={data.entities} /> */}
			<ProjectOverview
				project={data}
				metric={metric}
				setMetric={setMetric}
				rangeName={dateRange}
				graphClassName={styles.graph}
			/>
			<div className={styles.tables}>
				<DimTable project={data} dimension={"platform"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"browser"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"path"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"mobile"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"referrer"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"city"} metric={metric} range={resolveRange(dateRange).range} />
				<DimTable project={data} dimension={"country"} metric={metric} range={resolveRange(dateRange).range} />
			</div>
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

const DimTable = ({
	project,
	dimension,
	metric,
	range,
}: { project: ProjectResponse; dimension: Dimension; metric: Metric; range: DateRange }) => {
	const { data } = useQuery({
		placeholderData: (prev) => prev,
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

	const biggest = useMemo(() => data?.data?.reduce((acc, d) => Math.max(acc, d.value), 0) ?? 0, [data]);
	// e.g ["Chrome", "Firefox", "Safari"]
	const order = useMemo(() => data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue), [data]);

	return (
		<div>
			<div className={styles.dimTable}>
				<div className={styles.header}>
					<div>{dimensionNames[dimension]}</div>
					<div>{metricNames[metric]}</div>
				</div>
				{data?.data?.map((d) => {
					const value = metric === "avg_views_per_session" ? d.value / 1000 : d.value;
					const biggestVal = metric === "avg_views_per_session" ? biggest / 1000 : biggest;

					return (
						<div key={d.dimensionValue} style={{ order: order?.indexOf(d.dimensionValue) }} className={styles.row}>
							<DimensionValueBar value={value} biggest={biggestVal}>
								{dimension === "browser" && <BrowserIcon browser={d.dimensionValue} size={24} />}
								{dimension === "platform" && <OSIcon os={d.dimensionValue} size={24} />}
								&nbsp;
								{d.displayName ?? d.dimensionValue}
							</DimensionValueBar>

							<div>{value.toFixed(1).replace(/\.0$/, "") || "0"}</div>
						</div>
					);
				})}
			</div>
		</div>
	);
};

const DimensionValueBar = ({
	value,
	biggest,
	children,
}: { value: number; biggest: number; children?: React.ReactNode | React.ReactNode[] }) => {
	const percent = (value / biggest) * 100;
	return (
		<div
			className={styles.percentage}
			style={
				{
					"--percentage": `${percent}%`,
				} as React.CSSProperties
			}
		>
			{children}
		</div>
	);
};
