import * as Tabs from "@radix-ui/react-tabs";
import { FullscreenIcon, LinkIcon, ZoomIn } from "lucide-react";
import styles from "./dimensions.module.css";

import {
	type DateRange,
	type Dimension,
	type DimensionTableRow,
	type Metric,
	type ProjectResponse,
	dimensionNames,
	formatMetricVal,
	metricNames,
	useDimension,
} from "../../api";

import { cls } from "../../utils";
import { Dialog } from "../dialog";
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

const DetailsModal = ({
	project,
	dimension,
	metric,
	range,
}: { project: ProjectResponse; dimension: Dimension; metric: Metric; range: DateRange }) => {
	const { data, biggest, order, isLoading } = useDimension({ project, dimension, metric, range });

	return (
		<Dialog
			title={`${dimensionNames[dimension]} - ${metricNames[metric]}`}
			description={`Detailed breakdown of ${dimensionNames[dimension]} by ${metricNames[metric]}`}
			hideTitle
			hideDescription
			showClose
			className={styles.detailsModal}
			trigger={() => (
				<button
					type="button"
					className={cls(styles.showMore, (data?.length ?? 0) === 0 && styles.showMoreHidden)}
					onClick={() => console.log("show more")}
				>
					<ZoomIn size={16} />
					Show details
				</button>
			)}
		>
			<div className={styles.dimensionTable} style={{ "--count": data?.length } as React.CSSProperties}>
				<div className={styles.dimensionHeader}>
					<div>{dimensionNames[dimension]}</div>
					<div>{metricNames[metric]}</div>
				</div>
				{data?.map((d) => {
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
				{isLoading && data?.length === 0 && (
					<div className={styles.dimensionEmpty}>
						<div>Loading...</div>
					</div>
				)}
				{!isLoading && data?.length === 0 && (
					<div className={styles.dimensionEmpty}>
						<div>No data available</div>
					</div>
				)}
			</div>
		</Dialog>
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
			{value.displayName ?? "Unknown"}
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
