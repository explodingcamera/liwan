import { ZoomInIcon } from "lucide-react";
import fuzzysort from "fuzzysort";
import styles from "./dimensions.module.css";

import { cls, formatMetricVal } from "../../utils";
import { Dialog } from "../dialog";
import { DimensionLabel, DimensionValueBar } from ".";
import {
	dimensionNames,
	metricNames,
	useDimension,
	type DateRange,
	type Dimension,
	type Metric,
	type ProjectResponse,
} from "../../api";
import { useDeferredValue, useMemo, useState } from "react";

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
