import { ZoomInIcon } from "lucide-react";
import fuzzysort from "fuzzysort";
import styles from "./dimensions.module.css";

import { cls } from "../../utils";
import { Dialog } from "../dialog";
import { DimensionLabel, DimensionValueBar } from ".";
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
import { useDeferredValue, useEffect, useMemo, useState } from "react";

export const DetailsModal = ({
	project,
	dimension,
	metric,
	range,
}: { project: ProjectResponse; dimension: Dimension; metric: Metric; range: DateRange }) => {
	const { data, biggest, order, isLoading } = useDimension({ project, dimension, metric, range });

	const [query, setQuery] = useState("");
	const deferredQuery = useDeferredValue(query);

	const results = useMemo(() => {
		if (!deferredQuery || !data) return data;
		return fuzzysort.go(deferredQuery, data, { keys: ["displayName", "dimensionValue", "value"] }).map((r) => r.obj);
	}, [deferredQuery, data]);

	return (
		<Dialog
			title={`${dimensionNames[dimension]} - ${metricNames[metric]}`}
			description={`Detailed breakdown of ${dimensionNames[dimension]} by ${metricNames[metric]}`}
			hideTitle
			hideDescription
			showClose
			className={styles.detailsModal}
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
					<div>{metricNames[metric]}</div>
				</div>
				<input
					type="search"
					placeholder="Search..."
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					className={styles.search}
				/>
				{results?.map((d) => {
					const row = d as DimensionTableRow;
					return (
						<div
							key={d.dimensionValue}
							style={{ order: order?.indexOf(d.dimensionValue) }}
							className={styles.dimensionRow}
						>
							<DimensionValueBar value={d.value} biggest={biggest}>
								<DimensionLabel dimension={dimension} value={row} />
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
