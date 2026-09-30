import styles from "./dimensions.module.css";

import { useDeferredValue, useMemo, useState } from "react";
import fuzzysort from "fuzzysort";
import { XIcon, ZoomInIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Dimension, DimensionTableRow } from "@/constants";
import { dimensionNames, eventMetricName } from "@/constants";
import { useDimension } from "@/hooks/api";
import { cls, formatMetricVal } from "@/utils";
import type { ProjectQuery } from "..";
import { Breadcrumb, DimensionLabel, DimensionValueBar, isSelected } from ".";

export const DetailsModal = ({
	dimension,
	propertyKey,
	query,
	onSelect,
	onBack,
}: {
	dimension: Dimension;
	propertyKey?: string;
	query: ProjectQuery;
	onSelect?: (value: DimensionTableRow) => void;
	onBack?: () => void;
}) => {
	const { data, biggest, order, isLoading } = useDimension({
		dimension,
		propertyKey,
		...query,
	});
	const title = propertyKey ?? dimensionNames[dimension];

	const [filter, setFilter] = useState("");
	const deferredFilter = useDeferredValue(filter);

	const results = useMemo(() => {
		if (!deferredFilter || !data) return data;
		return fuzzysort
			.go(deferredFilter, data, {
				keys: ["displayName", "dimensionValue", "value"],
			})
			.map((r) => r.obj);
	}, [deferredFilter, data]);

	return (
		<Dialog
			title={`${title} by ${eventMetricName(query.metric, query.eventName)}`}
			description={`Detailed breakdown of ${title} by ${eventMetricName(query.metric, query.eventName)}.`}
			hideTitle
			hideDescription
			autoOverflow
			className={styles.detailsModal}
			trigger={
				<button type="button" className={cls(styles.showMore, (data?.length ?? 0) === 0 && styles.showMoreHidden)}>
					<ZoomInIcon size={16} />
					Show details
				</button>
			}
		>
			<div className={styles.dimensionTable} style={{ "--count": data?.length } as React.CSSProperties}>
				<div className={styles.dimensionHeader}>
					{onBack ? (
						<Breadcrumb parent={dimensionNames[dimension]} title={title} onBack={onBack} />
					) : (
						<div>{title}</div>
					)}
					<div>{eventMetricName(query.metric, query.eventName)}</div>
					<Dialog.Close className={styles.detailsClose} aria-label="Close dialog">
						<XIcon size={20} />
					</Dialog.Close>
				</div>
				<input
					type="search"
					placeholder="Search"
					value={filter}
					onChange={(e) => setFilter(e.target.value)}
					className={styles.search}
				/>
				{results?.map((d) => {
					return (
						<div
							key={d.dimensionValue}
							style={{ order: order?.indexOf(d.dimensionValue) }}
							className={styles.dimensionRow}
						>
							<DimensionValueBar
								value={d.value}
								biggest={biggest}
								selected={isSelected(query, dimension, d.dimensionValue, propertyKey)}
							>
								<DimensionLabel dimension={dimension} value={d} onSelect={onSelect} />
							</DimensionValueBar>
							<div>{formatMetricVal(d.value, query.metric)}</div>
						</div>
					);
				})}
				{isLoading && (
					<div className={styles.loadingOverlay} data-no-delay={!data}>
						<LoadingSpinner immediate />
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
