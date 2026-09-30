import styles from "./dimensions.module.css";

import { useDeferredValue, useMemo, useState } from "react";
import { XIcon, ZoomInIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { dimensionNames, eventMetricName } from "@/constants";
import { useDimension } from "@/hooks/api";
import { cls, fuzzyFilter } from "@/utils";
import { Breadcrumb, type DimensionProps, DimensionRow } from ".";

export const DetailsModal = (props: DimensionProps) => {
	const { dimension, propertyKey, query, onBack } = props;
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
		return fuzzyFilter(deferredFilter, data, ["displayName", "dimensionValue", "value"]);
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
				{results?.map((d) => (
					<DimensionRow key={d.dimensionValue} {...props} row={d} order={order} biggest={biggest} />
				))}
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
