import styles from "./filters.module.css";
import tagStyles from "./tags.module.css";

import { useState } from "react";
import { PlusIcon, Trash2Icon, XIcon } from "lucide-react";

import type { FilterOption, GenericFilter } from "@/components/dashboard/project/filter";
import { FilterDialog, filterOptions } from "@/components/dashboard/project/filter";
import type { DataRetention, GeoDetail, IngestDropRule, IngestFilter, VisitorGroupMode } from "@/constants";
import { filterNames, geoDetails, ingestDimensions, visitorGroupModes } from "@/constants";

const docsUrl = (hash: string) => `https://liwan.dev/collected-data/#${hash}`;
const title = (value: string) => value.replaceAll("_", " ").replace(/\b\w/g, (char) => char.toUpperCase());

const ingestFilterOptions: Record<string, FilterOption> = {
	event: {
		label: "Event",
		invertable: false,
		filterTypes: ["equal", "contains", "starts_with", "ends_with", "is_null"],
	},
	...filterOptions,
};

const ingestFilterTypeLabel = (filter: IngestFilter) =>
	ingestFilterOptions[filter.dimension]?.displayType?.({
		filterType: filter.filterType,
		inversed: false,
	}) ?? filterNames[filter.filterType];

const ingestFilterValueLabel = (filter: IngestFilter) =>
	ingestFilterOptions[filter.dimension]?.displayValue?.({
		filterType: filter.filterType,
		value: filter.value,
	}) ?? filter.value;

export const DocsLink = ({ hash }: { hash: string }) => (
	// biome-ignore lint/a11y/noAmbiguousAnchorText: short inline link text is intentional in these setting descriptions
	<a href={docsUrl(hash)} target="_blank" rel="noopener noreferrer">
		Learn more.
	</a>
);

export const VisitorModeSelect = ({
	id,
	value,
	onChange,
	allowInherit = false,
}: {
	id?: string;
	value?: VisitorGroupMode | null;
	onChange?: (value: VisitorGroupMode | null) => void;
	allowInherit?: boolean;
}) => (
	<select
		id={id}
		name="visitorGroupMode"
		value={value ?? "inherit"}
		onChange={(event) => {
			const next = event.currentTarget.value;
			if (next === "inherit" && allowInherit) onChange?.(null);
			if ((visitorGroupModes as readonly string[]).includes(next)) onChange?.(next as VisitorGroupMode);
		}}
	>
		{allowInherit && <option value="inherit">Inherit global</option>}
		<option value="accurate">Accurate</option>
		<option value="random_per_request">Random per request</option>
		<option value="network_standard">Network standard (/24 IPv4, /56 IPv6)</option>
		<option value="network_balanced">Network balanced (/28 IPv4, /64 IPv6)</option>
		<option value="network_accurate">Network accurate (full IP)</option>
	</select>
);

export const GeoSelect = ({
	id,
	value,
	onChange,
	allowInherit = false,
}: {
	id?: string;
	value?: GeoDetail | null;
	onChange?: (value: GeoDetail | null) => void;
	allowInherit?: boolean;
}) => (
	<select
		id={id}
		name="trackGeo"
		value={value ?? "inherit"}
		onChange={(event) => {
			const next = event.currentTarget.value;
			if (next === "inherit" && allowInherit) onChange?.(null);
			if ((geoDetails as readonly string[]).includes(next)) onChange?.(next as GeoDetail);
		}}
	>
		{allowInherit && <option value="inherit">Inherit global</option>}
		<option value="none">No geolocation lookup</option>
		<option value="country">Country only</option>
		<option value="city">Country and city</option>
	</select>
);

const retentionOptions = [
	{ value: "keep_all", label: "Keep all history" },
	{ value: "30", label: "1 month" },
	{ value: "90", label: "3 months" },
	{ value: "180", label: "6 months" },
	{ value: "365", label: "1 year" },
	{ value: "730", label: "2 years" },
];

export const RetentionSelect = ({
	value,
	onChange,
	allowInherit = false,
}: {
	value: DataRetention;
	onChange: (value: DataRetention) => void;
	allowInherit?: boolean;
}) => {
	const selected = () => {
		if (value.mode === "inherit" && allowInherit) return "inherit";
		if (value.mode !== "days") return "keep_all";
		const days = String(value.days);
		return retentionOptions.some((option) => option.value === days) ? days : "365";
	};

	return (
		<select
			name="historyRetention"
			value={selected()}
			onChange={(event) => {
				const next = event.currentTarget.value;
				if (next === "inherit") onChange({ mode: "inherit" });
				else if (next === "keep_all") onChange({ mode: "all" });
				else onChange({ mode: "days", days: Number(next) });
			}}
		>
			{allowInherit && <option value="inherit">Inherit global</option>}
			{retentionOptions.map((option) => (
				<option key={option.value} value={option.value}>
					{option.label}
				</option>
			))}
		</select>
	);
};

export const AllowedHostnamesEditor = ({
	value,
	onChange,
}: {
	value: string[];
	onChange: (value: string[]) => void;
}) => {
	const [hostname, setHostname] = useState("");
	const addHostname = () => {
		const next = hostname.trim();
		if (!next || value.includes(next)) return;
		onChange([...value, next]);
		setHostname("");
	};

	return (
		<div className={`${tagStyles.inputGroup} ${tagStyles.chips}`}>
			{value.map((hostname, index) => (
				<span className={tagStyles.chip} key={`${hostname}-${index}`}>
					{hostname}
					<button
						type="button"
						className={tagStyles.chipRemove}
						aria-label={`Remove ${hostname}`}
						onClick={() => onChange(value.filter((_, i) => i !== index))}
					>
						<XIcon size={14} />
					</button>
				</span>
			))}
			<input
				className={tagStyles.input}
				value={hostname}
				onChange={(event) => setHostname(event.currentTarget.value)}
				onKeyDown={(event) => {
					if (event.key !== "Enter" && event.key !== ",") return;
					event.preventDefault();
					addHostname();
				}}
				onBlur={addHostname}
				placeholder={value.length === 0 ? "example.com or *.example.com" : "Add hostname"}
				autoComplete="off"
			/>
		</div>
	);
};

export const FiltersEditor = ({
	rules,
	setRules,
	scope = "global",
}: {
	rules: IngestDropRule[];
	setRules: (rules: IngestDropRule[]) => void;
	scope?: "global" | "entity";
}) => {
	const [removingRule, setRemovingRule] = useState<number | null>(null);
	const removeRule = (index: number) => {
		if (removingRule !== null) return;
		if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
			setRules(rules.filter((_, i) => i !== index));
		} else {
			setRemovingRule(index);
		}
	};

	return (
		<section className={styles.section}>
			<div className={styles.sectionHeader}>
				<h2 className={styles.sectionTitle}>{scope === "entity" ? "Additional drop rules" : "Global drop rules"}</h2>
			</div>
			{scope === "entity" ? (
				<small className={styles.description}>
					Global drop rules still apply. Rules added here only apply to this entity. Within a rule, all filters must
					match. Matching any rule drops the event. <DocsLink hash="drop-rules" />
				</small>
			) : (
				<small className={styles.description}>
					Events are dropped before they are stored. Within a rule, all filters must match. Matching any rule drops the
					event. <DocsLink hash="drop-rules" />
				</small>
			)}
			{rules.length > 0 && (
				<div className={styles.ruleList}>
					{rules.map((rule, ruleIndex) => (
						<article
							className={styles.ruleCard}
							key={ruleIndex}
							data-removing={removingRule === ruleIndex || undefined}
							onAnimationEnd={(event) => {
								if (event.target !== event.currentTarget || removingRule !== ruleIndex) return;
								setRules(rules.filter((_, i) => i !== ruleIndex));
								setRemovingRule(null);
							}}
						>
							<div className={styles.ruleHeader}>
								<strong>Rule {ruleIndex + 1}</strong>
								<div className={styles.ruleActions}>
									<button
										type="button"
										className={styles.iconButton}
										aria-label="Remove rule"
										onClick={() => removeRule(ruleIndex)}
									>
										<Trash2Icon size={16} />
									</button>
								</div>
							</div>
							{rule.filters.length === 0 ? (
								<small>Add at least one filter to use this rule.</small>
							) : (
								<div className={styles.filterList}>
									{rule.filters.map((filter, filterIndex) => (
										<div className={styles.filterRow} key={`${filter.dimension}-${filterIndex}`}>
											<div className={styles.filterText}>
												<strong>{title(filter.dimension)}</strong>
												<span>{ingestFilterTypeLabel(filter)}</span>
												{filter.filterType !== "is_null" && (
													<span className={styles.filterValue}>{ingestFilterValueLabel(filter)}</span>
												)}
											</div>
											<button
												type="button"
												className={styles.iconButton}
												aria-label={`Remove ${title(filter.dimension)} filter`}
												onClick={() => {
													const next = [...rules];
													next[ruleIndex] = {
														filters: rule.filters.filter((_, i) => i !== filterIndex),
													};
													setRules(next);
												}}
											>
												<XIcon size={15} />
											</button>
										</div>
									))}
								</div>
							)}
							<div className={styles.ruleFooter}>
								<FilterDialog
									buttonText="Add filter"
									buttonIcon={<PlusIcon size={15} />}
									dimensions={[...ingestDimensions]}
									options={ingestFilterOptions}
									allowInverted={false}
									onAdd={(filter: GenericFilter) => {
										const next = [...rules];
										next[ruleIndex] = {
											filters: [
												...rule.filters,
												{
													dimension: filter.dimension,
													filterType: filter.filterType,
													value: filter.value,
												},
											],
										};
										setRules(next);
									}}
								/>
							</div>
						</article>
					))}
				</div>
			)}
			<div className={styles.addRuleAction}>
				<button type="button" onClick={() => setRules([...rules, { filters: [] }])}>
					<PlusIcon size={15} />
					<span>{rules.length === 0 ? "Add rule" : "Add another rule"}</span>
				</button>
			</div>
		</section>
	);
};
