import styles from "./filter.module.css";

import type { ReactElement } from "react";
import { useRef, useState } from "react";
import { SearchIcon, XIcon } from "lucide-react";

import type { DimensionFilter, FilterType } from "../../constants";
import { dimensionNames, filterNames, filterNamesInverted } from "../../constants";
import { capitalizeAll, cls } from "../../utils";
import { Dialog } from "../dialog";

export const SelectFilters = ({
	value,
	onChange,
	dimensions,
}: {
	value: DimensionFilter[];
	onChange: (filters: DimensionFilter[]) => void;
	dimensions?: string[];
}) => {
	const addFilter = (filter: GenericFilter) => {
		onChange([
			...value,
			{
				dimension: filter.dimension as DimensionFilter["dimension"],
				filterType: filter.filterType,
				value: filter.value,
				inversed: filter.inversed ?? false,
			},
		]);
	};

	return (
		<div className={styles.filters}>
			{value.map((filter, i) => (
				<article className={cls(styles.filter)} key={i}>
					<div className={styles.filterField}>
						<span>{dimensionNames[filter.dimension]}</span>
						<span className={styles.filterType}>
							{filterOptions[filter.dimension]?.displayType?.(filter) ??
								(filter.inversed ? filterNamesInverted[filter.filterType] : filterNames[filter.filterType])}
						</span>
						{filter.filterType === "is_null" ? null : (
							<span className={styles.filterValue}>
								{filterOptions[filter.dimension]?.displayValue?.(filter) ?? filter.value}
							</span>
						)}
					</div>
					<button type="button" onClick={() => onChange(value.filter((_, j) => i !== j))} className={styles.remove}>
						<XIcon size={20} />
					</button>
				</article>
			))}
			<article className={styles.filter}>
				<FilterDialog dimensions={dimensions} onAdd={addFilter} />
			</article>
		</div>
	);
};

export type FilterOption = {
	label: string;
	filterTypes?: readonly FilterType[];
	invertable?: boolean;
	custom?: boolean;
	render?: () => ReactElement;
	getFilter?: (data: FormData) => Pick<DimensionFilter, "filterType" | "value">;
	displayValue?: (filter: Pick<DimensionFilter, "filterType" | "value">) => string;
	displayType?: (filter: Pick<DimensionFilter, "filterType" | "inversed">) => string;
};

export const filterOptions: Record<string, FilterOption> = {
	platform: {
		label: dimensionNames.platform,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	browser: {
		label: dimensionNames.browser,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	url: {
		label: dimensionNames.url,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	url_entry: {
		label: dimensionNames.url_entry,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	url_exit: {
		label: dimensionNames.url_exit,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	fqdn: {
		label: dimensionNames.fqdn,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	path: {
		label: dimensionNames.path,
		invertable: true,
		filterTypes: ["equal", "contains", "starts_with", "ends_with"],
	},
	referrer: {
		label: dimensionNames.referrer,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	city: {
		label: dimensionNames.city,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	country: {
		label: dimensionNames.country,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_campaign: {
		label: dimensionNames.utm_campaign,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_content: {
		label: dimensionNames.utm_content,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_medium: {
		label: dimensionNames.utm_medium,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_source: {
		label: dimensionNames.utm_source,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	utm_term: {
		label: dimensionNames.utm_term,
		invertable: true,
		filterTypes: ["equal", "contains"],
	},
	mobile: {
		label: dimensionNames.mobile,
		custom: true,
		displayValue: (filter) => (filter.filterType === "is_true" ? "Mobile" : "Desktop"),
		displayType: (filter) => (filter.inversed ? "is not" : "is"),
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
				filterType: data.get("mobile") === "true" ? "is_true" : "is_false",
				value: undefined,
			};
		},
	},
};

const displayFilters = Object.keys(filterOptions).filter(
	(dimension) => dimension !== "url_entry" && dimension !== "url_exit",
);

export type GenericFilter = {
	dimension: string;
	filterType: FilterType;
	value?: string | null;
	inversed?: boolean;
};

export const FilterDialog = ({
	onAdd,
	dimensions = displayFilters,
	options = filterOptions,
	allowInverted = true,
	buttonText = "Add Filter",
	buttonIcon = <SearchIcon size={20} />,
}: {
	onAdd: (filter: GenericFilter) => void;
	dimensions?: string[];
	options?: Record<string, FilterOption>;
	allowInverted?: boolean;
	buttonText?: string;
	buttonIcon?: ReactElement | null;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const selectableDimensions = dimensions.filter((dimension) => options[dimension]);
	const [dimension, setDimension] = useState(selectableDimensions[0] ?? "url");
	const selectedDimension = options[dimension] ? dimension : (selectableDimensions[0] ?? "");
	const filter = options[selectedDimension];
	if (!filter) return null;

	const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
		e.preventDefault();
		const data = new FormData(e.currentTarget);

		if (filter.getFilter) {
			onAdd({ dimension: selectedDimension, ...filter.getFilter(data) });
			closeRef.current?.click();
			return;
		}

		const filterType = data.get("filterType");
		if (typeof filterType !== "string" || !(filter.filterTypes as readonly string[] | undefined)?.includes(filterType))
			return;

		const value = data.get("value");
		onAdd({
			dimension: selectedDimension,
			inversed: filter.invertable && data.get("show-matches") === "inverted",
			filterType: filterType as FilterType,
			value: typeof value === "string" ? value : null,
		});
		setDimension(selectableDimensions[0] ?? "url");
		closeRef.current?.click();
	};

	return (
		<Dialog
			title="Add Filter"
			description="Filter the report by a specific dimension."
			hideDescription
			trigger={
				<button type="button">
					<span>{buttonText}</span>
					{buttonIcon}
				</button>
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Dimension
					<select name="dimension" value={selectedDimension} onChange={(e) => setDimension(e.target.value)}>
						{selectableDimensions.map((dimension) => (
							<option key={dimension} value={dimension}>
								{options[dimension].label}
							</option>
						))}
					</select>
				</label>

				{filter.custom && filter.render?.()}

				{!filter.custom && (
					<div className={styles.formInvertable}>
						<label>
							Filter type
							<select name="filterType">
								{filter.filterTypes?.map((filterType) => (
									<option key={filterType} value={filterType}>
										{capitalizeAll(filterNames[filterType])}
									</option>
								))}
							</select>
						</label>
						{allowInverted && filter.invertable && (
							<div className={styles.inverted}>
								<fieldset>
									<label>
										<input name="show-matches" defaultChecked value="default" type="radio" aria-invalid="false" />
										Show matches
									</label>
									<label>
										<input name="show-matches" type="radio" value="inverted" aria-invalid="true" />
										Exclude matches
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
					<Dialog.Close ref={closeRef} className="secondary outline">
						Cancel
					</Dialog.Close>
					<button type="submit">Add filter</button>
				</div>
			</form>
		</Dialog>
	);
};
