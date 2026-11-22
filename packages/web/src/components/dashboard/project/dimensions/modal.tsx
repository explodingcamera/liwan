import styles from "./dimensions.module.css";

import { useDeferredValue, useMemo, useState } from "react";
import { XIcon, ZoomInIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { type DimensionTableRow, dimensionNames, eventMetricName } from "@/constants";
import type { useDimension } from "@/hooks/api";
import { cls, fuzzyFilter } from "@/utils";
import { Breadcrumb, type DimensionProps, DimensionRow } from ".";

export const DetailsModal = ({ report, ...props }: DimensionProps & { report: ReturnType<typeof useDimension> }) => {
	const { dimension, propertyKey, query, onBack } = props;
	const { data, biggest, isLoading, error } = report;
	const title = propertyKey ?? dimensionNames[dimension];
	const metric = eventMetricName(query.metric, query.eventName);

	return (
		<ReportDetailsDialog
			title={onBack ? <Breadcrumb parent={dimensionNames[dimension]} title={title} onBack={onBack} /> : title}
			metric={metric}
			description={`Detailed breakdown of ${title} by ${metric}.`}
			dialogTitle={`${title} by ${metric}`}
			data={data}
			isLoading={isLoading}
			error={error}
			renderRow={(row) => <DimensionRow key={row.dimensionValue} {...props} row={row} biggest={biggest} />}
			trigger={
				<button type="button" className={cls(styles.showMore, (data?.length ?? 0) === 0 && styles.showMoreHidden)}>
					<ZoomInIcon size={16} />
					Show details
				</button>
			}
		/>
	);
};

export const ReportDetailsDialog = <Row extends DimensionTableRow>({
	title,
	dialogTitle,
	description,
	metric,
	data,
	isLoading,
	error,
	emptyMessage = "No data available",
	renderRow,
	trigger,
	open,
	onOpenChange,
}: {
	title: React.ReactNode;
	dialogTitle: string;
	description: string;
	metric: string;
	data?: Row[];
	isLoading: boolean;
	error?: unknown;
	emptyMessage?: string;
	renderRow: (row: Row) => React.ReactNode;
	trigger: React.ReactElement | false;
	open?: boolean;
	onOpenChange?: (open: boolean) => void;
}) => {
	const [localOpen, setLocalOpen] = useState(false);
	const isOpen = open ?? localOpen;
	const [filter, setFilter] = useState("");
	const deferredFilter = useDeferredValue(filter);

	const results = useMemo(() => {
		if (!isOpen) return undefined;
		if (!deferredFilter || !data) return data;
		return fuzzyFilter(deferredFilter, data, ["displayName", "dimensionValue", "value"]);
	}, [isOpen, deferredFilter, data]);

	return (
		<Dialog
			title={dialogTitle}
			description={description}
			hideTitle
			hideDescription
			autoOverflow
			className={styles.detailsModal}
			trigger={trigger}
			open={isOpen}
			onOpenChange={onOpenChange ?? setLocalOpen}
		>
			<div className={styles.dimensionTable} style={{ "--count": data?.length } as React.CSSProperties}>
				<div className={styles.dimensionHeader}>
					{typeof title === "string" ? <div>{title}</div> : title}
					<div className={styles.metricHeading}>{metric}</div>
					<Dialog.Close className={styles.detailsClose} aria-label="Close dialog">
						<XIcon size={20} />
					</Dialog.Close>
				</div>
				<input
					type="search"
					aria-label="Search report"
					placeholder="Search"
					value={filter}
					onChange={(e) => setFilter(e.target.value)}
					className={styles.search}
				/>
				<div className={styles.detailRows} aria-busy={isLoading}>
					{results?.map(renderRow)}
					{isLoading && (
						<div className={styles.loadingOverlay} data-no-delay={!data}>
							<LoadingSpinner immediate />
						</div>
					)}
					{!isLoading && (!!error || results?.length === 0) && (
						<div className={styles.dimensionEmpty}>
							<div>
								{error ? "Could not load this breakdown." : data?.length ? "No matching results" : emptyMessage}
							</div>
						</div>
					)}
				</div>
			</div>
		</Dialog>
	);
};
