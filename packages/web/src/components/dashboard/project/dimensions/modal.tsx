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
import { DimensionLabel, DimensionValueBar, isSelected } from ".";

export const DetailsModal = ({
	dimension,
	query,
	onSelect,
}: {
	dimension: Dimension;
	query: ProjectQuery;
	onSelect?: (value: DimensionTableRow) => void;
}) => {
	const { data, biggest, order, isLoading } = useDimension({
		dimension,
		...query,
	});

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
			title={`${dimensionNames[dimension]} by ${eventMetricName(query.metric, query.eventName)}`}
			description={`Detailed breakdown of ${dimensionNames[dimension]} by ${eventMetricName(query.metric, query.eventName)}.`}
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
			<div
				className={cls(styles.dimensionTable, isLoading && styles.loading)}
				style={{ "--count": data?.length } as React.CSSProperties}
			>
				<div className={styles.dimensionHeader}>
					<div>{dimensionNames[dimension]}</div>
					<div>{eventMetricName(query.metric, query.eventName)}</div>
					<Dialog.Close className={styles.detailsClose} aria-label="Close dialog">
						<XIcon size={22} />
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
								selected={isSelected(query, dimension, d.dimensionValue)}
							>
								<DimensionLabel dimension={dimension} value={d} onSelect={onSelect} />
							</DimensionValueBar>
							<div>{formatMetricVal(d.value, query.metric)}</div>
						</div>
					);
				})}
				{isLoading && <LoadingSpinner className={styles.spinner} />}
				{!isLoading && data?.length === 0 && (
					<div className={styles.dimensionEmpty}>
						<div>No data available</div>
					</div>
				)}
			</div>
		</Dialog>
	);
};
