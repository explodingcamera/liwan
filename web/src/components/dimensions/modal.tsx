import fuzzysort from "fuzzysort";
import { ZoomInIcon } from "lucide-react";
import styles from "./dimensions.module.css";

import { useDeferredValue, useMemo, useState } from "react";
import { DimensionLabel, DimensionValueBar } from ".";
import { type Dimension, type DimensionTableRow, dimensionNames, metricNames, useDimension } from "../../api";
import { cls, formatMetricVal } from "../../utils";
import { Dialog } from "../dialog";
import type { ProjectQuery } from "../project";

export const DetailsModal = ({
	dimension,
	query,
	onSelect,
}: {
	dimension: Dimension;
	query: ProjectQuery;
	onSelect: (value: DimensionTableRow, dimension: Dimension) => void;
}) => {
	const { data, biggest, order, isLoading } = useDimension({ dimension, ...query });

	const [filter, setFilter] = useState("");
	const deferredFilter = useDeferredValue(filter);

	const results = useMemo(() => {
		if (!deferredFilter || !data) return data;
		return fuzzysort.go(deferredFilter, data, { keys: ["displayName", "dimensionValue", "value"] }).map((r) => r.obj);
	}, [deferredFilter, data]);

	return (
		<Dialog
			title={`${dimensionNames[dimension]} - ${metricNames[query.metric]}`}
			description={`Detailed breakdown of ${dimensionNames[dimension]} by ${metricNames[query.metric]}`}
			hideTitle
			hideDescription
			showClose
			className={styles.detailsModal}
			autoOverflow
			trigger={() => (
				<button type="button" className={cls(styles.showMore, (data?.length ?? 0) === 0 && styles.showMoreHidden)}>
					<ZoomInIcon size={16} />
					Show details
				</button>
			)}
		>
			<div className={styles.dimensionTable} style={{ "--count": data?.length } as React.CSSProperties}>
				<div className={styles.dimensionHeader}>
					<div>{dimensionNames[dimension]}</div>
					<div>{metricNames[query.metric]}</div>
				</div>
				<input
					type="search"
					placeholder="Search..."
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
							<DimensionValueBar value={d.value} biggest={biggest}>
								<DimensionLabel dimension={dimension} value={d} onSelect={() => onSelect(d, dimension)} />
							</DimensionValueBar>
							<div>{formatMetricVal(d.value, query.metric)}</div>
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
