import * as Tabs from "@radix-ui/react-tabs";
import { LinkIcon, SquareArrowOutUpRightIcon } from "lucide-react";
import styles from "./dimensions.module.css";

import { type Dimension, type DimensionTableRow, dimensionNames, metricNames, useDimension } from "../../api";

import { BrowserIcon, MobileDeviceIcon, OSIcon, ReferrerIcon } from "../icons";
import { cls, countryCodeToFlag, formatHost, formatPath, getHref, tryParseUrl } from "../../utils";
import { DetailsModal } from "./modal";
import { formatMetricVal } from "../../utils";
import type { ProjectQuery } from "../project";

export const cardStyles = styles.card;

type DimensionProps = {
	dimension: Dimension;
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow) => void;
};

export const DimensionCard = (props: DimensionProps) => {
	return (
		<article className={styles.card}>
			<div className={styles.dimensionHeader}>
				<div>{dimensionNames[props.dimension]}</div>
				<div>{metricNames[props.query.metric]}</div>
			</div>
			<DimensionTable {...props} />
		</article>
	);
};

export const DimensionTabsCard = ({
	dimensions,
	query,
	onSelect,
}: {
	dimensions: Dimension[];
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	return (
		<article className={styles.card}>
			<DimensionTabs dimensions={dimensions} query={query} onSelect={onSelect} />
		</article>
	);
};

export const DimensionTabs = ({
	dimensions,
	query,
	onSelect,
}: {
	dimensions: Dimension[];
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	return (
		<Tabs.Root className={styles.tabs} defaultValue={dimensions[0]}>
			<Tabs.List className={styles.tabsList}>
				{Object.entries(dimensions).map(([key, value]) => (
					<Tabs.Trigger key={key} value={value}>
						{dimensionNames[value]}
					</Tabs.Trigger>
				))}
				<div>{metricNames[query.metric]}</div>
			</Tabs.List>
			{dimensions.map((dimension) => (
				<Tabs.Content key={dimension} value={dimension} className={styles.tabsContent}>
					<DimensionTable dimension={dimension} query={query} onSelect={(value) => onSelect(value, dimension)} />
				</Tabs.Content>
			))}
		</Tabs.Root>
	);
};

export const DimensionTable = (props: DimensionProps) => {
	const { data, biggest, order, isLoading } = useDimension({ dimension: props.dimension, ...props.query });
	const dataTruncated = data?.slice(0, 6);

	return (
		<>
			<div
				className={cls(styles.dimensionTable, isLoading && styles.loading)}
				style={{ "--count": 6 } as React.CSSProperties}
			>
				{isLoading && <div className={cls("loading-spinner", styles.spinner)} />}
				{dataTruncated?.map((d) => {
					return (
						<div
							key={d.dimensionValue}
							style={{ order: order?.indexOf(d.dimensionValue) }}
							className={styles.dimensionRow}
						>
							<DimensionValueBar value={d.value} biggest={biggest}>
								<DimensionLabel dimension={props.dimension} value={d} onSelect={props.onSelect} />
							</DimensionValueBar>
							<div>{formatMetricVal(d.value)}</div>
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
			<DetailsModal {...props} />
		</>
	);
};

const DimensionValueButton = ({
	children,
	onSelect,
}: {
	children: React.ReactNode;
	onSelect: () => void;
}) => (
	<button type="button" className={styles.dimensionItemSelect} onClick={onSelect}>
		{children}
	</button>
);

const dimensionLabels: Record<Dimension, (value: DimensionTableRow, onSelect: () => void) => React.ReactNode> = {
	platform: (value, onSelect) => (
		<>
			<OSIcon os={value.dimensionValue} size={24} />
			<DimensionValueButton onSelect={onSelect}>{value.dimensionValue}</DimensionValueButton>
		</>
	),
	browser: (value, onSelect) => (
		<>
			<BrowserIcon browser={value.dimensionValue} size={24} />
			<DimensionValueButton onSelect={onSelect}>{value.dimensionValue}</DimensionValueButton>
		</>
	),
	url: (value, onSelect) => {
		const url = tryParseUrl(value.dimensionValue);

		return (
			<>
				<LinkIcon size={16} />
				<DimensionValueButton onSelect={onSelect}>{formatPath(url)}</DimensionValueButton>
				<a href={getHref(url)} target="_blank" rel="noreferrer" className={styles.external}>
					<SquareArrowOutUpRightIcon size={16} />
				</a>
				{typeof url !== "string" && <span className={styles.hostname}>{formatHost(url)}</span>}
			</>
		);
	},
	fqdn: (value, onSelect) => {
		const url = tryParseUrl(value.dimensionValue);
		return (
			<>
				<LinkIcon size={16} />
				<DimensionValueButton onSelect={onSelect}>{formatHost(url)}</DimensionValueButton>
				<a href={getHref(url)} target="_blank" rel="noreferrer" className={styles.external}>
					<SquareArrowOutUpRightIcon size={16} />
				</a>
			</>
		);
	},
	mobile: (value, onSelect) => (
		<>
			<MobileDeviceIcon isMobile={value.dimensionValue === "true"} size={24} />
			<DimensionValueButton onSelect={onSelect}>
				{value.dimensionValue === "true" ? "Mobile" : "Desktop"}
			</DimensionValueButton>
		</>
	),
	country: (value, onSelect) => (
		<>
			<span>{countryCodeToFlag(value.dimensionValue)}</span>
			<DimensionValueButton onSelect={onSelect}>
				{value.displayName || value.dimensionValue || "Unknown"}
			</DimensionValueButton>
		</>
	),
	city: (value, onSelect) => (
		<>
			<span>{countryCodeToFlag(value.icon || "XX")}</span>
			<DimensionValueButton onSelect={onSelect}>{value.displayName || "Unknown"}</DimensionValueButton>
		</>
	),
	referrer: (value, onSelect) => (
		<>
			<ReferrerIcon referrer={value.dimensionValue} icon={value.icon} size={24} />
			<DimensionValueButton onSelect={onSelect}>
				{value.displayName || value.dimensionValue || "Unknown"}
			</DimensionValueButton>
			{value.dimensionValue && isValidFqdn(value.dimensionValue) && (
				<a href={`https://${value.dimensionValue}`} target="_blank" rel="noreferrer" className={styles.external}>
					<SquareArrowOutUpRightIcon size={16} />
				</a>
			)}
		</>
	),
	path: (value, onSelect) => (
		<>
			<LinkIcon size={16} />
			<DimensionValueButton onSelect={onSelect}>{value.dimensionValue}</DimensionValueButton>
		</>
	),
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

export const DimensionLabel = ({
	dimension,
	value,
	onSelect,
}: { dimension: Dimension; value: DimensionTableRow; onSelect: (value: DimensionTableRow) => void }) =>
	dimensionLabels[dimension](value, () => onSelect(value));

export const DimensionValueBar = ({
	value,
	biggest,
	children,
}: { value: number; biggest: number; children?: React.ReactNode }) => (
	<div className={styles.percentage} style={{ "--percentage": `${(value / biggest) * 100}%` } as React.CSSProperties}>
		{children}
	</div>
);
