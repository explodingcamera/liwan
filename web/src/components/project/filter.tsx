import styles from "./filter.module.css";
import { SearchIcon, XIcon } from "lucide-react";
import {
	capitalizeAll,
	dimensionNames,
	filterNames,
	filterNamesInverted,
	filterTypes,
	type DimensionFilter,
	type FilterType,
} from "../../api";
import { Dialog } from "../dialog";
import { cls } from "../../utils";
import { useRef, useState } from "react";

export const SelectFilters = ({
	value,
	onChange,
}: {
	value: DimensionFilter[];
	onChange: (filters: DimensionFilter[]) => void;
}) => {
	return (
		<div className={styles.filters}>
			{value.map((filter, i) => (
				<article className={cls(styles.filter)} key={i}>
					<div className={styles.filterField}>
						<span>{dimensionNames[filter.dimension]}</span>
						<span className={styles.filterType}>
							{filter.inversed ? filterNamesInverted[filter.filterType] : filterNames[filter.filterType]}
						</span>
						{filter.filterType === "is_null" ? null : <span className={styles.filterValue}>{filter.value}</span>}
					</div>
					<button type="button" onClick={() => onChange(value.filter((_, j) => i !== j))} className={styles.remove}>
						<XIcon size={20} />
					</button>
				</article>
			))}
			<article className={styles.filter}>
				<FilterDialog onAdd={(filter) => onChange([...value, filter])} />
			</article>
		</div>
	);
};

const FilterDialog = ({
	onAdd,
}: {
	onAdd: (filter: DimensionFilter) => void;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const [dimension, setDimension] = useState<keyof typeof dimensionNames>("url");
	const [filterType, setFilterType] = useState<FilterType | `${FilterType}-inverted`>("equal");
	const [value, setValue] = useState("");

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		onAdd({
			dimension,
			inversed: filterType.endsWith("-inverted"),
			filterType: filterType.replace("-inverted", "") as FilterType,
			value: value.length ? value : undefined,
		});
		setDimension("url");
		setFilterType("equal");
		setValue("");
		closeRef.current?.click();
	};

	const dimensions = Object.entries(dimensionNames) as [keyof typeof dimensionNames, string][];

	return (
		<Dialog
			title="Add Filter"
			trigger={
				<button type="button">
					<h2>Add Filter</h2>
					<SearchIcon size={20} />
				</button>
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Dimension
					<select
						name="dimension"
						value={dimension}
						onChange={(e) => setDimension(e.target.value as keyof typeof dimensionNames)}
					>
						{dimensions.map(([dimension, name]) => (
							<option key={dimension} value={dimension}>
								{dimensionNames[dimension]}
							</option>
						))}
					</select>
				</label>
				<label>
					Filter Type
					<select
						name="filterType"
						value={filterType}
						onChange={(e) => setFilterType(e.target.value as FilterType | `${FilterType}-inverted`)}
					>
						{filterTypes.map((filterType) => (
							<>
								<option key={filterType} value={filterType}>
									{capitalizeAll(filterNames[filterType])}
								</option>
								<option key={`${filterType}-inverted`} value={`${filterType}-inverted`}>
									{capitalizeAll(filterNamesInverted[filterType])}
								</option>
							</>
						))}
					</select>
				</label>
				<label>
					Value
					<input type="text" name="value" value={value} onChange={(e) => setValue(e.target.value)} />
				</label>
				<div className="grid">
					<Dialog.Close asChild ref={closeRef}>
						<button className="secondary outline" type="button">
							Cancel
						</button>
					</Dialog.Close>
					<button type="submit">Add Filter</button>
				</div>
			</form>
		</Dialog>
	);
};
