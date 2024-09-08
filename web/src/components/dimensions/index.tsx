import * as Tabs from "@radix-ui/react-tabs";
import { LinkIcon, SquareArrowOutUpRightIcon, ZoomIn } from "lucide-react";
import styles from "./dimensions.module.css";

import {
	type DateRange,
	type Dimension,
	type DimensionTableRow,
	type Metric,
	type ProjectResponse,
	dimensionNames,
	metricNames,
	useDimension,
} from "../../api";

import { BrowserIcon, MobileDeviceIcon, OSIcon, ReferrerIcon } from "../icons";
import { countryCodeToFlag, formatFullUrl, formatHost, getHref, tryParseUrl } from "../../utils";
import { DetailsModal } from "./modal";
import { formatMetricVal } from "../../utils";

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
		<article className={styles.card}>
			<div className={styles.dimensionHeader}>
				<div>{dimensionNames[dimension]}</div>
				<div>{metricNames[metric]}</div>
			</div>
			<DimensionTable project={project} dimension={dimension} metric={metric} range={range} />
		</article>
	);
};

export const DimensionTabsCard = ({
	project,
	metric,
	range,
	dimensions,
}: { project: ProjectResponse; dimensions: Dimension[]; metric: Metric; range: DateRange }) => {
	return (
		<article className={styles.card}>
			<DimensionTabs project={project} dimensions={dimensions} metric={metric} range={range} />
		</article>
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
				<Tabs.Content key={dimension} value={dimension} className={styles.tabsContent}>
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
	const { data, biggest, order, isLoading } = useDimension({ project, dimension, metric, range });

	const dataTruncated = data?.slice(0, 6);
	return (
		<>
			<div className={styles.dimensionTable} style={{ "--count": 6 } as React.CSSProperties}>
				{dataTruncated?.map((d) => {
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
				{/* {isLoading && dataTruncated?.length === 0 && (
				)} */}
				{!isLoading && dataTruncated?.length === 0 && (
					<div className={styles.dimensionEmpty}>
						<div>No data available</div>
					</div>
				)}
			</div>
			<DetailsModal project={project} dimension={dimension} metric={metric} range={range} />
		</>
	);
};

const dimensionLabels: Record<Dimension, (value: DimensionTableRow) => React.ReactNode> = {
	platform: (value) => (
		<>
			<OSIcon os={value.dimensionValue} size={24} />
			{value.dimensionValue}
		</>
	),
	browser: (value) => (
		<>
			<BrowserIcon browser={value.dimensionValue} size={24} />
			{value.dimensionValue}
		</>
	),
	url: (value) => {
		const url = tryParseUrl(value.dimensionValue);

		return (
			<>
				<LinkIcon size={16} />
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
				<a target="_blank" rel="noreferrer" href={getHref(url)}>
					{formatHost(url)}
				</a>
			</>
		);
	},
	mobile: (value) => (
		<>
			<MobileDeviceIcon isMobile={value.dimensionValue === "true"} size={24} />
			{value.dimensionValue === "true" ? "Mobile" : "Desktop"}
		</>
	),
	country: (value) => (
		<>
			<span>{countryCodeToFlag(value.dimensionValue)}</span>
			{value.displayName || value.dimensionValue || "Unknown"}
		</>
	),
	city: (value) => (
		<>
			<span>{countryCodeToFlag(value.icon || "XX")}</span>
			{value.displayName || "Unknown"}
		</>
	),
	referrer: (value) => (
		<>
			<ReferrerIcon referrer={value.dimensionValue} icon={value.icon} size={24} />
			{value.displayName || value.dimensionValue || "Unknown"}
			{value.dimensionValue && isValidFqdn(value.dimensionValue) && (
				<>
					<a href={`https://${value.dimensionValue}`} target="_blank" rel="noreferrer" className={styles.external}>
						<SquareArrowOutUpRightIcon size={16} />
					</a>
				</>
			)}
		</>
	),
	path: (value) => value.dimensionValue,
};

const isValidFqdn = (fqdn: string) => {
	if (!fqdn.includes(".")) return false;
	try {
		new URL(`https://${fqdn}`);
		return true;
	} catch {
		return false;
	}
};

export const DimensionLabel = ({ dimension, value }: { dimension: Dimension; value: DimensionTableRow }) =>
	dimensionLabels[dimension](value);

export const DimensionValueBar = ({
	value,
	biggest,
	children,
}: { value: number; biggest: number; children?: React.ReactNode }) => (
	<div className={styles.percentage} style={{ "--percentage": `${(value / biggest) * 100}%` } as React.CSSProperties}>
		{children}
	</div>
);
