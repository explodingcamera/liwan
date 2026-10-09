import styles from "./filter.module.css";

import type { ReactElement, SubmitEvent } from "react";
import { Fragment, useRef, useState } from "react";
import { SearchIcon, XIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import type { DimensionFilter, FilterType } from "@/constants";
import { dimensionNames, filterNames, filterNamesInverted } from "@/constants";
import { capitalizeAll } from "@/utils";

export const SelectFilters = ({
	value,
	onChange,
	dimensions,
	eventName,
	onClearEvent,
}: {
	value: DimensionFilter[];
	onChange: (filters: DimensionFilter[]) => void;
	dimensions?: string[];
	eventName?: string;
	onClearEvent?: () => void;
}) => {
	const addFilter = (filter: GenericFilter) => {
		onChange([
			...value,
			{
				dimension: filter.dimension as DimensionFilter["dimension"],
				key: filter.key,
				filterType: filter.filterType,
				value: filter.value,
				inversed: filter.inversed ?? false,
			},
		]);
	};

	return (
		<div className={styles.filters}>
			{eventName && eventName !== "pageview" && (
				<article className={styles.filter}>
					<div className={styles.filterField}>
						<span>Event</span>
						<span className={styles.filterType}>is</span>
						<span className={styles.filterValue}>{eventName}</span>
					</div>
					<button type="button" aria-label="Remove event filter" onClick={onClearEvent} className={styles.remove}>
						<XIcon size={20} />
					</button>
				</article>
			)}
			{value.map((filter, i) => (
				<article className={styles.filter} key={i}>
					<div className={styles.filterField}>
						<span>{filter.dimension === "property" ? filter.key : dimensionNames[filter.dimension]}</span>
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
					<button
						type="button"
						aria-label={`Remove ${dimensionNames[filter.dimension]} filter`}
						onClick={() => onChange(value.filter((_, j) => i !== j))}
						className={styles.remove}
					>
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
	hasKey?: boolean;
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
		filterTypes: ["contains", "equal"],
	},
	browser: {
		label: dimensionNames.browser,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	url: {
		label: dimensionNames.url,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	url_entry: {
		label: dimensionNames.url_entry,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	url_exit: {
		label: dimensionNames.url_exit,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	fqdn: {
		label: dimensionNames.fqdn,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	path: {
		label: dimensionNames.path,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with"],
	},
	referrer: {
		label: dimensionNames.referrer,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	city: {
		label: dimensionNames.city,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	country: {
		label: dimensionNames.country,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	utm_campaign: {
		label: dimensionNames.utm_campaign,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	utm_content: {
		label: dimensionNames.utm_content,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	utm_medium: {
		label: dimensionNames.utm_medium,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	utm_source: {
		label: dimensionNames.utm_source,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	utm_term: {
		label: dimensionNames.utm_term,
		invertable: true,
		filterTypes: ["contains", "equal"],
	},
	property: {
		label: "Property",
		hasKey: true,
		invertable: true,
		filterTypes: ["contains", "equal", "starts_with", "ends_with", "is_null"],
		displayType: (filter) => {
			if (filter.filterType === "is_null") return filter.inversed ? "is set" : "is not set";
			if (filter.filterType === "equal") return filter.inversed ? "is not" : "is";
			return filter.inversed ? filterNamesInverted[filter.filterType] : filterNames[filter.filterType];
		},
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
	key?: string;
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
	const [selectedCondition, setSelectedCondition] = useState<string>();
	const selectedDimension = options[dimension] ? dimension : (selectableDimensions[0] ?? "");
	const filter = options[selectedDimension];
	if (!filter) return null;
	const selectedFilterType = selectedCondition?.replace(/^not:/, "") as FilterType | undefined;
	const filterType =
		selectedFilterType && filter.filterTypes?.includes(selectedFilterType)
			? selectedFilterType
			: filter.filterTypes?.[0];
	const condition = filterType
		? `${selectedCondition?.startsWith("not:") && allowInverted && filter.invertable ? "not:" : ""}${filterType}`
		: undefined;

	const handleSubmit = (e: SubmitEvent<HTMLFormElement>) => {
		e.preventDefault();
		const data = new FormData(e.currentTarget);

		if (filter.getFilter) {
			onAdd({ dimension: selectedDimension, ...filter.getFilter(data) });
			closeRef.current?.click();
			return;
		}

		const selected = data.get("condition");
		if (typeof selected !== "string") return;
		const inversed = selected.startsWith("not:");
		const filterType = inversed ? selected.slice(4) : selected;
		if (!(filter.filterTypes as readonly string[] | undefined)?.includes(filterType)) return;
		if (inversed && (!allowInverted || !filter.invertable)) return;

		const value = data.get("value");
		const key = data.get("key");
		onAdd({
			dimension: selectedDimension,
			key: filter.hasKey && typeof key === "string" ? key.trim() : undefined,
			inversed,
			filterType: filterType as FilterType,
			value: typeof value === "string" ? value : null,
		});
		closeRef.current?.click();
	};

	return (
		<Dialog
			onOpenChange={(open) => {
				if (open) return;
				setDimension(selectableDimensions[0] ?? "url");
				setSelectedCondition(undefined);
			}}
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

				{filter.hasKey && (
					<label>
						Key *
						<input type="text" name="key" required maxLength={64} pattern=".*\S.*" />
					</label>
				)}

				{!filter.custom && (
					<label>
						Condition
						<select
							name="condition"
							value={condition}
							onChange={(event) => setSelectedCondition(event.currentTarget.value)}
						>
							{filter.filterTypes?.map((type) => (
								<Fragment key={type}>
									<option value={type}>
										{capitalizeAll(filter.displayType?.({ filterType: type, inversed: false }) ?? filterNames[type])}
									</option>
									{allowInverted && filter.invertable && (
										<option value={`not:${type}`}>
											{capitalizeAll(
												filter.displayType?.({ filterType: type, inversed: true }) ?? filterNamesInverted[type],
											)}
										</option>
									)}
								</Fragment>
							))}
						</select>
					</label>
				)}

				{!filter.custom && filterType !== "is_null" && (
					<label>
						Value *
						<input type="text" name="value" required />
					</label>
				)}

				<div className="action-row">
					<Dialog.Close ref={closeRef} className="button-secondary">
						Cancel
					</Dialog.Close>
					<button type="submit">Add filter</button>
				</div>
			</form>
		</Dialog>
	);
};
