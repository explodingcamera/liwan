import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import styles from "./project.module.css";

import {
	api,
	dimensionNames,
	formatMetricVal,
	metricNames,
	useQuery,
	type DateRange,
	type Dimension,
	type DimensionTableRow,
	type Metric,
	type ProjectResponse,
} from "../api";
import { ProjectOverview } from "./projects";
import { useLocalStorage } from "@uidotdev/usehooks";
import { resolveRange, type RangeName } from "../api/ranges";
import { BrowserIcon, MobileDeviceIcon, OSIcon, ReferrerIcon } from "./icons";
import { LinkIcon } from "lucide-react";
const server = typeof window === "undefined";

const WorldMap = lazy(() => import("./worldmap").then((module) => ({ default: module.WorldMap })));

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
				<Card>
					<DimTable project={data} dimension={"platform"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>

				<Card>
					<DimTable project={data} dimension={"browser"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card fullWidth>
					<GeoCard project={data} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"url"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"fqdn"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"mobile"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"referrer"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"city"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
				<Card>
					<DimTable project={data} dimension={"country"} metric={metric} range={resolveRange(dateRange).range} />
				</Card>
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

const Card = ({ children, fullWidth }: { children: React.ReactNode; fullWidth?: boolean }) => {
	return (
		<div className={styles.card} data-full-width={fullWidth ?? undefined}>
			<Suspense>{children}</Suspense>
		</div>
	);
};

const GeoCard = ({ project, metric, range }: { project: ProjectResponse; metric: Metric; range: DateRange }) => {
	const { data } = useQuery({
		placeholderData: (prev) => prev,
		queryKey: ["dimension", project.id, "country", metric, range],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/dimension"]
				.post({
					params: { project_id: project.id },
					json: {
						dimension: "country",
						metric,
						range,
					},
				})
				.json(),
	});

	const biggest = useMemo(() => data?.data?.reduce((acc, d) => Math.max(acc, d.value), 0) ?? 0, [data]);
	const order = useMemo(() => data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue), [data]);

	return (
		<div className={styles.geoCard}>
			<div>
				<WorldMap data={data?.data} metric={metric} />
			</div>
			<div>
				{data?.data?.map((d) => {
					const value = metric === "avg_views_per_session" ? d.value / 1000 : d.value;
					const biggestVal = metric === "avg_views_per_session" ? biggest / 1000 : biggest;

					return (
						<div key={d.dimensionValue} style={{ order: order?.indexOf(d.dimensionValue) }} className={styles.dimRow}>
							<DimensionValueBar value={value} biggest={biggestVal}>
								<DimensionLabel dimension={"country"} value={d} />
							</DimensionValueBar>

							<div>{value.toFixed(1).replace(/\.0$/, "") || "0"}</div>
						</div>
					);
				})}
			</div>
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
	const order = useMemo(() => data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue), [data]);

	return (
		<div className={styles.dimTable}>
			<div className={styles.header}>
				<div>{dimensionNames[dimension]}</div>
				<div>{metricNames[metric]}</div>
			</div>
			{data?.data?.map((d) => {
				const value = d.value;
				const biggestVal = biggest;

				return (
					<div key={d.dimensionValue} style={{ order: order?.indexOf(d.dimensionValue) }} className={styles.dimRow}>
						<DimensionValueBar value={value} biggest={biggestVal}>
							<DimensionLabel dimension={dimension} value={d} />
						</DimensionValueBar>

						<div>{formatMetricVal(metric, value)}</div>
					</div>
				);
			})}
		</div>
	);
};

const DimensionLabel = ({ dimension, value }: { dimension: Dimension; value: DimensionTableRow }) => {
	if (dimension === "platform")
		return (
			<>
				<OSIcon os={value.dimensionValue} size={24} />
				&nbsp;
				{value.dimensionValue}
			</>
		);

	if (dimension === "browser")
		return (
			<>
				<BrowserIcon browser={value.dimensionValue} size={24} />
				&nbsp;
				{value.dimensionValue}
			</>
		);

	if (dimension === "url") {
		const url = value.dimensionValue;
		return (
			<>
				<LinkIcon size={16} />
				&nbsp;
				<a href={value.dimensionValue}>{url}</a>
			</>
		);
	}

	if (dimension === "fqdn") {
		const url = tryParseUrl(value.dimensionValue);
		return (
			<>
				<LinkIcon size={16} />
				&nbsp;
				<a href={value.dimensionValue}>{url.hostname}</a>
			</>
		);
	}

	if (dimension === "mobile")
		return (
			<>
				<MobileDeviceIcon isMobile={value.dimensionValue === "true"} size={24} />
				&nbsp;
				{value.dimensionValue === "true" ? "Mobile" : "Desktop"}
			</>
		);

	if (dimension === "country") {
		return (
			<>
				{countryCodeToFlag(value.dimensionValue)}
				&nbsp;
				{value.displayName ?? value.dimensionValue ?? "Unknown"}
			</>
		);
	}

	if (dimension === "city") {
		console.log(value);

		return (
			<>
				{countryCodeToFlag(value.icon || "XX")}
				&nbsp;
				{value.displayName ?? value.dimensionValue ?? "Unknown"}
			</>
		);
	}

	if (dimension === "referrer") {
		return (
			<>
				<ReferrerIcon referrer={value.dimensionValue} icon={value.icon} size={24} />
				&nbsp;
				{value.displayName ?? value.dimensionValue ?? "Unknown"}
			</>
		);
	}

	return <>{value.displayName ?? value.dimensionValue ?? "Unknown"}</>;
};

const tryParseUrl = (url: string) => {
	try {
		return new URL(url);
	} catch {
		return new URL(`https://${url}`);
	}
};

const countryCodeToFlag = (countryCode: string) => {
	const code = countryCode.length === 2 ? countryCode : "XX";
	const codePoints = code
		.toUpperCase()
		.split("")
		.map((char) => 127397 + char.charCodeAt(0));
	return String.fromCodePoint(...codePoints);
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
