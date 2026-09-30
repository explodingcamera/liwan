import styles from "./dimensions/dimensions.module.css";

import type { Dispatch, SetStateAction } from "react";

import type { DimensionFilter } from "@/constants";
import { dimensionNames, eventMetricName } from "@/constants";
import { getDimensionFilter, type ProjectQuery } from ".";
import { Breadcrumb, DimensionTable } from "./dimensions";

const isOpenKeyFilter = (filter: DimensionFilter) =>
	filter.dimension === "property" && filter.filterType === "is_null" && filter.inversed;

/** Lists property keys; opening a key filters by `key is set` and lists its values. */
export const PropertiesCard = ({
	query,
	onSelect,
	setFilters,
}: {
	query: ProjectQuery;
	onSelect: (filter: DimensionFilter) => void;
	setFilters: Dispatch<SetStateAction<DimensionFilter[]>>;
}) => {
	const propertyKey = query.filters.findLast(isOpenKeyFilter)?.key ?? undefined;
	const closeKey = () =>
		setFilters((filters) => filters.filter((filter) => !isOpenKeyFilter(filter) || filter.key !== propertyKey));

	return (
		<article className={styles.card}>
			<div className={styles.dimensionHeader}>
				{propertyKey ? (
					<Breadcrumb parent={dimensionNames.property} title={propertyKey} onBack={closeKey} />
				) : (
					<div>{dimensionNames.property}</div>
				)}
				<div>{eventMetricName(query.metric, query.eventName)}</div>
			</div>
			<DimensionTable
				dimension="property"
				propertyKey={propertyKey}
				query={query}
				onBack={propertyKey ? closeKey : undefined}
				onSelect={(value) =>
					propertyKey
						? onSelect(getDimensionFilter("property", value.dimensionValue, propertyKey))
						: setFilters((filters) => [
								...filters,
								{ dimension: "property", key: value.dimensionValue, filterType: "is_null", inversed: true },
							])
				}
			/>
		</article>
	);
};
