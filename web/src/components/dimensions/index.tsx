import styles from "./dimensions.module.css";
import { LinkIcon } from "lucide-react";
import * as Tabs from "@radix-ui/react-tabs";

import {
	dimensionNames,
	formatMetricVal,
	metricNames,
	useDimension,
	type DateRange,
	type Dimension,
	type DimensionTableRow,
	type Metric,
	type ProjectResponse,
} from "../../api";

import { BrowserIcon, MobileDeviceIcon, OSIcon, ReferrerIcon } from "../icons";
import { countryCodeToFlag, formatFullUrl, formatHost, getHref, tryParseUrl } from "./utils";

export const cardStyles = styles.card;

export const DimensionCard = ({
	project,
	dimension,
	metric,
	range,
}: {
	project: ProjectResponse;
	dimension: Dimension;
	metric: Metric;
	range: DateRange;
}) => {
	return (
		<div className={styles.card}>
			<div className={styles.dimensionHeader}>
				<div>{dimensionNames[dimension]}</div>
				<div>{metricNames[metric]}</div>
			</div>
			<DimensionTable project={project} dimension={dimension} metric={metric} range={range} />
		</div>
	);
};

export const DimensionTabsCard = ({
	project,
	metric,
	range,
	dimensions,
}: { project: ProjectResponse; dimensions: Dimension[]; metric: Metric; range: DateRange }) => {
	return (
		<div className={styles.card}>
			<DimensionTabs project={project} dimensions={dimensions} metric={metric} range={range} />
		</div>
	);
};

export const DimensionTabs = ({
	project,
	metric,
	range,
	dimensions,
}: { project: ProjectResponse; dimensions: Dimension[]; metric: Metric; range: DateRange }) => {
	return (
		<Tabs.Root className={styles.tabs} defaultValue={dimensions[0]}>
			<Tabs.List className={styles.tabsList}>
				{Object.entries(dimensions).map(([key, value]) => (
					<Tabs.Trigger key={key} value={value}>
						{dimensionNames[value]}
					</Tabs.Trigger>
				))}
				<div>{metricNames[metric]}</div>
			</Tabs.List>
			{dimensions.map((dimension) => (
				<Tabs.Content key={dimension} value={dimension}>
					<DimensionTable dimension={dimension} metric={metric} range={range} project={project} noHeader />
				</Tabs.Content>
			))}
		</Tabs.Root>
	);
};

export const DimensionTable = ({
	project,
	dimension,
	metric,
	range,
}: { project: ProjectResponse; dimension: Dimension; metric: Metric; range: DateRange; noHeader?: boolean }) => {
	const { data, biggest, order } = useDimension({ project, dimension, metric, range });
	return <DimensionList value={data ?? []} dimension={dimension} metric={metric} biggest={biggest} order={order} />;
};

export const DimensionList = ({
	value,
	dimension,
	metric,
	biggest,
	order,
}: {
	value: DimensionTableRow[];
	dimension: Dimension;
	metric: Metric;
	biggest: number;
	order?: string[];
}) => {
	return (
		<div>
			{value.map((d) => {
				return (
					<div
						key={d.dimensionValue}
						style={{ order: order?.indexOf(d.dimensionValue) }}
						className={styles.dimensionRow}
					>
						<DimensionValueBar value={d.value} biggest={biggest}>
							<DimensionLabel dimension={dimension} value={d} />
						</DimensionValueBar>
						<div>{formatMetricVal(metric, d.value)}</div>
					</div>
				);
			})}
		</div>
	);
};

const dimensionLabels: Record<Dimension, (value: DimensionTableRow) => React.ReactNode> = {
	platform: (value) => (
		<>
			<OSIcon os={value.dimensionValue} size={24} />
			&nbsp;
			{value.dimensionValue}
		</>
	),
	browser: (value) => (
		<>
			<BrowserIcon browser={value.dimensionValue} size={24} />
			&nbsp;
			{value.dimensionValue}
		</>
	),
	url: (value) => {
		const url = tryParseUrl(value.dimensionValue);

		return (
			<>
				<LinkIcon size={16} />
				&nbsp;
				<a target="_blank" rel="noreferrer" href={getHref(url)}>
					{formatFullUrl(url)}
				</a>
			</>
		);
	},
	fqdn: (value) => {
		const url = tryParseUrl(value.dimensionValue);
		return (
			<>
				<LinkIcon size={16} />
				&nbsp;
				<a target="_blank" rel="noreferrer" href={getHref(url)}>
					{formatHost(url)}
				</a>
			</>
		);
	},
	mobile: (value) => (
		<>
			<MobileDeviceIcon isMobile={value.dimensionValue === "true"} size={24} />
			&nbsp;
			{value.dimensionValue === "true" ? "Mobile" : "Desktop"}
		</>
	),
	country: (value) => (
		<>
			{countryCodeToFlag(value.dimensionValue)}
			&nbsp;
			{value.displayName ?? value.dimensionValue ?? "Unknown"}
		</>
	),
	city: (value) => (
		<>
			{countryCodeToFlag(value.icon || "XX")}
			&nbsp;
			{value.displayName ?? value.dimensionValue ?? "Unknown"}
		</>
	),
	referrer: (value) => (
		<>
			<ReferrerIcon referrer={value.dimensionValue} icon={value.icon} size={24} />
			&nbsp;
			{value.displayName ?? value.dimensionValue ?? "Unknown"}
		</>
	),
	path: (value) => value.dimensionValue,
};

const DimensionLabel = ({ dimension, value }: { dimension: Dimension; value: DimensionTableRow }) =>
	dimensionLabels[dimension](value);

const DimensionValueBar = ({
	value,
	biggest,
	children,
}: { value: number; biggest: number; children?: React.ReactNode }) => (
	<div className={styles.percentage} style={{ "--percentage": `${(value / biggest) * 100}%` } as React.CSSProperties}>
		{children}
	</div>
);
