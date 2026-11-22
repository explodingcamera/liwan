import styles from "./dimensions.module.css";

import { useEffect, useState } from "react";
import { Tabs } from "@base-ui/react/tabs";
import {
	ChevronRightIcon,
	LaptopIcon,
	LinkIcon,
	MonitorIcon,
	PinIcon,
	SmartphoneIcon,
	SquareArrowOutUpRightIcon,
	TabletIcon,
	TagIcon,
} from "lucide-react";

import { LoadingSpinner } from "@/components/ui/loading";
import type { Dimension, DimensionTableRow, Metric } from "@/constants";
import { dimensionNames, eventMetricName } from "@/constants";
import { useDimension } from "@/hooks/api";
import { cls, formatHost, formatPath, getHref, tryParseUrl } from "@/utils";
import { getDimensionFilter, type ProjectQuery } from "..";
import type { ComparisonRow } from "../comparison";
import { BrowserIcon, CountryFlag, MobileDeviceIcon, OrientationIcon, OSIcon, ReferrerIcon } from "../icons";
import { formatMetricVal, isMetricImprovement } from "../metric-format";
import { DimensionMetricValue } from "./metric-value";
import { DetailsModal } from "./modal";
import { ReportPagination, useReportPagination } from "./pagination";

export type DimensionProps = {
	dimension: Dimension;
	propertyKey?: string;
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow) => void;
	/** Shows a chevron next to each row to open it, e.g. to list a property key's values */
	onOpen?: (value: DimensionTableRow) => void;
	onBack?: () => void;
};

export const DimensionCard = (props: DimensionProps) => {
	return (
		<article className={styles.card}>
			<div className={styles.dimensionHeader}>
				<div>{dimensionNames[props.dimension]}</div>
				<div className={styles.metricHeading}>{eventMetricName(props.query.metric, props.query.eventName)}</div>
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

export const PageDimensionTabsCard = ({
	dimensions = ["url", "url_entry", "url_exit", "fqdn"],
	query,
	onSelect,
}: {
	dimensions?: Dimension[];
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	const showSessionPageDimensions = query.metric === "views" || query.metric === "unique_visitors";

	if (!showSessionPageDimensions || dimensions.length === 1) {
		const dimension = dimensions.includes("url") ? "url" : dimensions[0];
		return <DimensionCard dimension={dimension} query={query} onSelect={(value) => onSelect(value, dimension)} />;
	}

	return <DimensionDropdownCard dimensions={dimensions} query={query} onSelect={onSelect} />;
};

export const DimensionDropdownCard = ({
	dimensions,
	query,
	onSelect,
}: {
	dimensions: Dimension[];
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	const [selectedDimension, setSelectedDimension] = useState(dimensions[0]);

	useEffect(() => {
		if (!dimensions.includes(selectedDimension)) setSelectedDimension(dimensions[0]);
	}, [dimensions, selectedDimension]);

	return (
		<article className={styles.card}>
			<Tabs.Root
				className={styles.tabs}
				value={selectedDimension}
				onValueChange={(value) => setSelectedDimension(value as Dimension)}
			>
				<Tabs.List className={styles.tabsList}>
					<select
						className={styles.dimensionSelect}
						value={selectedDimension}
						onChange={(e) => setSelectedDimension(e.target.value as Dimension)}
					>
						{dimensions.map((dimension) => (
							<option key={dimension} value={dimension}>
								{dimensionNames[dimension]}
							</option>
						))}
					</select>
					<div className={styles.metricHeading}>{eventMetricName(query.metric, query.eventName)}</div>
				</Tabs.List>
				{dimensions.map((dimension) => (
					<Tabs.Panel key={dimension} value={dimension} className={styles.tabsContent}>
						<DimensionTable dimension={dimension} query={query} onSelect={(value) => onSelect(value, dimension)} />
					</Tabs.Panel>
				))}
			</Tabs.Root>
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
		<Tabs.Root key={dimensions.join(":")} className={styles.tabs} defaultValue={dimensions[0]}>
			<Tabs.List className={styles.tabsList}>
				{Object.entries(dimensions).map(([key, value]) => (
					<Tabs.Tab key={key} value={value}>
						{dimensionNames[value]}
					</Tabs.Tab>
				))}
				<div className={styles.metricHeading}>{eventMetricName(query.metric, query.eventName)}</div>
			</Tabs.List>
			{dimensions.map((dimension) => (
				<Tabs.Panel key={dimension} value={dimension} className={styles.tabsContent}>
					<DimensionTable dimension={dimension} query={query} onSelect={(value) => onSelect(value, dimension)} />
				</Tabs.Panel>
			))}
		</Tabs.Root>
	);
};

export const DimensionTable = (props: DimensionProps) => {
	const report = useDimension({
		dimension: props.dimension,
		propertyKey: props.propertyKey,
		...props.query,
	});
	const { data, biggest, isLoading, error } = report;
	const { pageRows, hasPages, previous, next } = useReportPagination(
		data,
		props.query,
		`${props.dimension}:${props.propertyKey ?? ""}`,
	);

	return (
		<>
			<div className={styles.dimensionTable} aria-busy={isLoading}>
				{isLoading && (
					<div className={styles.loadingOverlay} data-no-delay={!data}>
						<LoadingSpinner immediate />
					</div>
				)}
				{pageRows?.map((d) => (
					<DimensionRow key={d.dimensionValue} {...props} row={d} biggest={biggest} />
				))}
				{!isLoading && (error || pageRows?.length === 0) && (
					<div className={styles.dimensionEmpty}>
						<div>{error ? "Could not load this breakdown." : "No data available"}</div>
					</div>
				)}
			</div>
			<div className={styles.cardFooter}>
				<DetailsModal {...props} report={report} />
				{hasPages && !error && <ReportPagination previous={previous} next={next} />}
			</div>
		</>
	);
};

export const Breadcrumb = ({ parent, title, onBack }: { parent: string; title: string; onBack: () => void }) => (
	<div className={styles.breadcrumb}>
		<button type="button" onClick={onBack}>
			{parent}
		</button>
		<ChevronRightIcon size={16} aria-hidden="true" />
		<span title={title}>{title}</span>
	</div>
);

export const DimensionValueButton = ({ children, onSelect }: { children: string; onSelect?: () => void }) =>
	onSelect ? (
		<button type="button" className={styles.dimensionItemSelect} onClick={onSelect}>
			<span title={children}>{children}</span>
		</button>
	) : (
		<span className={styles.dimensionItemLabel} title={children}>
			{children}
		</span>
	);

const ExternalLink = ({ href, label }: { href?: string; label: string }) =>
	href && (
		<a
			href={href}
			target="_blank"
			rel="noreferrer"
			className={styles.external}
			aria-label={`Open ${label} in a new tab`}
		>
			<SquareArrowOutUpRightIcon size={16} />
		</a>
	);

const renderUtmDimensionLabel = (value: DimensionTableRow, onSelect?: () => void) => (
	<>
		<PinIcon size={24} />
		<DimensionValueButton onSelect={onSelect}>{value.dimensionValue || "Unknown/None"}</DimensionValueButton>
	</>
);

const renderUrlDimensionLabel = ({ dimensionValue }: DimensionTableRow, onSelect?: () => void) => {
	const url = tryParseUrl(dimensionValue);
	const path = formatPath(url) || "(unknown page)";

	return (
		<>
			<LinkIcon size={16} />
			<DimensionValueButton onSelect={dimensionValue ? onSelect : undefined}>{path}</DimensionValueButton>
			{dimensionValue && <ExternalLink href={getHref(url)} label={dimensionValue} />}
			{typeof url !== "string" && <span className={styles.hostname}>{formatHost(url)}</span>}
		</>
	);
};

const screenWidthRanges: Record<string, [number, number]> = {
	xs: [0, 479],
	sm: [480, 767],
	md: [768, 1023],
	lg: [1024, 1279],
	xl: [1280, 1535],
	"2xl": [1536, Number.POSITIVE_INFINITY],
};

const screenWidthIcons = new Map([
	["xs", SmartphoneIcon],
	["sm", SmartphoneIcon],
	["md", TabletIcon],
	["lg", LaptopIcon],
]);

const dimensionLabels: Record<Dimension, (value: DimensionTableRow, onSelect?: () => void) => React.ReactNode> = {
	utm_campaign: renderUtmDimensionLabel,
	utm_content: renderUtmDimensionLabel,
	utm_medium: renderUtmDimensionLabel,
	utm_source: renderUtmDimensionLabel,
	utm_term: renderUtmDimensionLabel,
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
	url: renderUrlDimensionLabel,
	url_entry: renderUrlDimensionLabel,
	url_exit: renderUrlDimensionLabel,
	fqdn: (value, onSelect) => {
		const url = tryParseUrl(value.dimensionValue);
		return (
			<>
				<LinkIcon size={16} />
				<DimensionValueButton onSelect={onSelect}>{formatHost(url)}</DimensionValueButton>
				<ExternalLink href={getHref(url)} label={value.dimensionValue} />
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
			<CountryFlag code={value.dimensionValue} />
			<DimensionValueButton onSelect={onSelect}>
				{value.displayName || value.dimensionValue || "Unknown"}
			</DimensionValueButton>
		</>
	),
	city: (value, onSelect) => (
		<>
			<CountryFlag code={value.icon} />
			<DimensionValueButton onSelect={onSelect}>{value.displayName || "Unknown"}</DimensionValueButton>
		</>
	),
	referrer: (value, onSelect) => {
		let name = value.displayName || value.dimensionValue || "Unknown";

		if (!value.dimensionValue.endsWith(".com") && name === "Google") {
			const end = value.dimensionValue.split(".").at(-1);
			name = `Google (${end})`;
		}

		return (
			<>
				<ReferrerIcon referrer={value.dimensionValue} icon={value.icon} size={24} />
				<DimensionValueButton onSelect={onSelect}>{name}</DimensionValueButton>
				{value.dimensionValue && isValidFqdn(value.dimensionValue) && (
					<ExternalLink href={`https://${value.dimensionValue}`} label={value.dimensionValue} />
				)}
			</>
		);
	},
	path: (value, onSelect) => (
		<>
			<LinkIcon size={16} />
			<DimensionValueButton onSelect={onSelect}>{value.dimensionValue}</DimensionValueButton>
		</>
	),
	property: (value, onSelect) => (
		<>
			<TagIcon size={16} />
			<DimensionValueButton onSelect={onSelect}>{value.dimensionValue}</DimensionValueButton>
		</>
	),
	screen_width: (value, onSelect) => {
		const key = value.dimensionValue.toLowerCase();
		const Icon = screenWidthIcons.get(key) ?? MonitorIcon;
		const range = Object.hasOwn(screenWidthRanges, key) ? screenWidthRanges[key] : undefined;
		return (
			<>
				<Icon size={16} aria-hidden="true" />
				<DimensionValueButton onSelect={onSelect}>
					{range ? key.toUpperCase() : value.dimensionValue || "Unknown"}
				</DimensionValueButton>
				{range && (
					<span className={styles.screenWidthRange}>
						({range[0]}px to {Number.isFinite(range[1]) ? `${range[1]}px` : "∞"})
					</span>
				)}
			</>
		);
	},
	orientation: (value, onSelect) => (
		<>
			<OrientationIcon orientation={value.dimensionValue} size={24} />
			<DimensionValueButton onSelect={onSelect}>
				{value.displayName ? value.displayName : "Unknown"}
			</DimensionValueButton>
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

export const DimensionRow = ({
	row,
	biggest,
	dimension,
	propertyKey,
	query,
	onSelect,
	onOpen,
}: Pick<DimensionProps, "dimension" | "propertyKey" | "onOpen"> & {
	row: ComparisonRow;
	biggest: number;
	query: Pick<ProjectQuery, "filters" | "metric">;
	onSelect?: DimensionProps["onSelect"];
}) => (
	<div
		className={styles.dimensionRow}
		title={
			row.comparisonValue === undefined
				? undefined
				: `Comparison: ${formatMetricVal(row.comparisonValue, query.metric)}`
		}
	>
		<DimensionValueBar
			value={row.value}
			comparisonValue={row.comparisonValue}
			biggest={biggest}
			metric={query.metric}
			selected={isSelected(query, dimension, row.dimensionValue, propertyKey)}
		>
			{dimensionLabels[dimension](row, onSelect && (() => onSelect(row)))}
			{onOpen && (
				<button
					type="button"
					className={styles.openButton}
					aria-label={`Open ${row.dimensionValue}`}
					title={`Open ${row.dimensionValue}`}
					onClick={() => onOpen(row)}
				>
					<ChevronRightIcon size={20} />
				</button>
			)}
		</DimensionValueBar>
		<DimensionMetricValue value={row.value} comparisonValue={row.comparisonValue} metric={query.metric} />
	</div>
);

export const DimensionValueBar = ({
	value,
	biggest,
	metric,
	children,
	selected = false,
	comparisonValue,
}: {
	value?: number | null;
	biggest: number;
	metric: Metric;
	children?: React.ReactNode;
	selected?: boolean;
	comparisonValue?: number | null;
}) => {
	const hasChange = value != null && comparisonValue != null && value !== comparisonValue;
	const end = hasChange ? Math.max(value, comparisonValue) : (value ?? 0);
	const overlap = hasChange && end > 0 ? Math.min(value, comparisonValue) / end : 1;
	const changeIsGood = hasChange && isMetricImprovement(value - comparisonValue, metric);

	return (
		<div
			className={cls(styles.percentage, selected && styles.selected)}
			data-empty={end <= 0}
			data-change={hasChange ? (changeIsGood ? "better" : "worse") : undefined}
			style={
				{
					"--percentage": `${biggest > 0 ? (end / biggest) * 100 : 0}%`,
					"--overlap": `${overlap * 100}%`,
				} as React.CSSProperties
			}
		>
			{children}
		</div>
	);
};

export const isSelected = (
	query: Pick<ProjectQuery, "filters">,
	dimension: Dimension,
	value: string,
	propertyKey?: string,
) => {
	// in the property key list, a key is active while any filter uses it
	if (dimension === "property" && propertyKey === undefined) {
		return query.filters.some((filter) => filter.dimension === "property" && filter.key === value);
	}
	const selected = getDimensionFilter(dimension, value, propertyKey);
	return query.filters.some(
		(filter) =>
			filter.dimension === selected.dimension &&
			filter.key === selected.key &&
			filter.filterType === selected.filterType &&
			filter.value === selected.value &&
			!filter.inversed,
	);
};
