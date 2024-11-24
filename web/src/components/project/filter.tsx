import { SearchIcon, XIcon } from "lucide-react";
import styles from "./filter.module.css";

import { type DimensionFilter, type FilterType, dimensionNames, filterNames, filterNamesInverted } from "../../api";

import { useRef, useState } from "react";
import { capitalizeAll, cls } from "../../utils";
import { Dialog } from "../dialog";

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
							{filters?.[filter.dimension].displayType?.(filter) ??
								(filter.inversed ? filterNamesInverted[filter.filterType] : filterNames[filter.filterType])}
						</span>
						{filter.filterType === "is_null" ? null : (
							<span className={styles.filterValue}>
								{filters?.[filter.dimension].displayValue?.(filter) ?? filter.value}
							</span>
						)}
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

const filters = {
	platform: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	browser: {
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	url: {
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	fqdn: {
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	path: {
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	referrer: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	city: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	country: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_campaign: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_content: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_medium: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_source: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_term: {
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	mobile: {
		custom: true,
		displayValue: (filter: DimensionFilter) => (filter.filterType === "is_true" ? "Mobile" : "Desktop"),
		displayType: (filter: DimensionFilter) => (filter.inversed ? "is not" : "is"),
		render: () => (
			<label>
				Device Type
				<select name="mobile">
					<option value="true">Mobile</option>
					<option value="false">Desktop</option>
				</select>
			</label>
		),
		getFilter: (data: FormData) => {
			return {
				dimension: "mobile",
				filterType: data.get("mobile") === "true" ? "is_true" : "is_false",
				value: undefined,
			};
		},
	},
} as Record<
	keyof typeof dimensionNames,
	{
		filterTypes: FilterType[];
		invertable?: boolean;
		custom?: boolean;
		render?: () => JSX.Element;
		getFilter?: (data: FormData) => DimensionFilter;
		displayValue?: (filter: DimensionFilter) => string;
		displayType?: (filter: DimensionFilter) => string;
	}
>;

type filterDimension = keyof typeof filters;

const FilterDialog = ({
	onAdd,
}: {
	onAdd: (filter: DimensionFilter) => void;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const [dimension, setDimension] = useState<filterDimension>("url");
	const filter = filters[dimension];

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		const data = new FormData(e.currentTarget as HTMLFormElement);

		if (filter.getFilter) {
			onAdd(filter.getFilter(data));
			closeRef.current?.click();
			return;
		}

		onAdd({
			dimension,
			inversed: filter.invertable && data.get("show-matches") === "inverted",
			filterType: data.get("filterType") as FilterType,
			value: data.get("value") as string,
		});
		setDimension("url");
		closeRef.current?.click();
	};

	return (
		<Dialog
			title="Add Filter"
			description="Filter the data by a specific dimension"
			hideDescription
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
					<select name="dimension" value={dimension} onChange={(e) => setDimension(e.target.value as filterDimension)}>
						{Object.keys(filters).map((dimension) => (
							<option key={dimension} value={dimension}>
								{dimensionNames[dimension as filterDimension]}
							</option>
						))}
					</select>
				</label>

				{filter.custom && filter.render?.()}

				{!filter.custom && (
					<div className={styles.formInvertable}>
						<label>
							Filter Type
							<select name="filterType">
								{filter.filterTypes.map((filterType) => (
									<option key={filterType} value={filterType}>
										{capitalizeAll(filterNames[filterType])}
									</option>
								))}
							</select>
						</label>
						{filter.invertable && (
							<div className={styles.inverted}>
								<fieldset>
									<label>
										<input name="show-matches" defaultChecked value="default" type="radio" aria-invalid="false" />
										Show Matches
									</label>
									<label>
										<input name="show-matches" type="radio" value="inverted" aria-invalid="true" />
										Exclude Matches
									</label>
								</fieldset>
							</div>
						)}
					</div>
				)}

				{!filter.custom && (
					<label>
						Value
						<input type="text" name="value" />
					</label>
				)}

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
