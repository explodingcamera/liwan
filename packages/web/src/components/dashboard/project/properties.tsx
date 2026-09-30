import styles from "./dimensions/dimensions.module.css";

import { type Dispatch, type SetStateAction, useState } from "react";

import type { DimensionFilter } from "@/constants";
import { dimensionNames, eventMetricName } from "@/constants";
import { getDimensionFilter, type ProjectQuery } from ".";
import { Breadcrumb, DimensionTable } from "./dimensions";

const usesKey = (filter: DimensionFilter, key: string) => filter.dimension === "property" && filter.key === key;

/**
 * Lists property keys. Selecting a key filters by `key is set` or clears its filters if it has any,
 * and its chevron lists the key's values.
 */
export const PropertiesCard = ({
	query,
	onSelect,
	setFilters,
}: {
	query: ProjectQuery;
	onSelect: (filter: DimensionFilter) => void;
	setFilters: Dispatch<SetStateAction<DimensionFilter[]>>;
}) => {
	const [propertyKey, setPropertyKey] = useState<string>();
	const closeKey = () => setPropertyKey(undefined);
	const toggleKey = (key: string) =>
		setFilters((filters) =>
			filters.some((filter) => usesKey(filter, key))
				? filters.filter((filter) => !usesKey(filter, key))
				: [...filters, { dimension: "property", key, filterType: "is_null", inversed: true }],
		);

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
				onOpen={propertyKey ? undefined : (value) => setPropertyKey(value.dimensionValue)}
				onSelect={(value) =>
					propertyKey
						? onSelect(getDimensionFilter("property", value.dimensionValue, propertyKey))
						: toggleKey(value.dimensionValue)
				}
			/>
		</article>
	);
};
