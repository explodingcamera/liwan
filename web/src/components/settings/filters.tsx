import styles from "./filters.module.css";

import { useState } from "react";
import { Trash2Icon, XIcon } from "lucide-react";

import type { FilterOption, GenericFilter } from "@/components/dashboard/project/filter";
import { FilterDialog, filterOptions } from "@/components/dashboard/project/filter";
import type { GeoDetail, IngestDropRule, IngestFilter, VisitorGroupMode } from "@/constants";
import { geoDetails, ingestDimensions, visitorGroupModes } from "@/constants";

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
	}) ?? title(filter.filterType);

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
		<div className={styles.hostnameEditor}>
			<div className={styles.hostnameBox}>
				{value.map((hostname, index) => (
					<span className={styles.hostnameChip} key={`${hostname}-${index}`}>
						{hostname}
						<button
							type="button"
							className={styles.hostnameChipRemove}
							aria-label={`Remove ${hostname}`}
							onClick={() => onChange(value.filter((_, i) => i !== index))}
						>
							<XIcon size={14} />
						</button>
					</span>
				))}
				<input
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
			<small className={styles.hostnameHelp}>
				Type a hostname and press Enter. Supports exact hostnames and <code>*.</code> wildcards in the subdomain
				position.
			</small>
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
}) => (
	<section className={styles.section}>
		<div className={styles.sectionHeader}>
			<h2 className={styles.sectionTitle}>{scope === "entity" ? "Additional drop rules" : "Global drop rules"}</h2>
			<div className={styles.filterAction}>
				<button type="button" onClick={() => setRules([...rules, { filters: [] }])}>
					Add rule
				</button>
			</div>
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
		{rules.length === 0 ? (
			<p className={styles.empty}>No drop rules yet.</p>
		) : (
			<div className={styles.ruleList}>
				{rules.map((rule, ruleIndex) => (
					<article className={styles.ruleCard} key={ruleIndex}>
						<div className={styles.ruleHeader}>
							<div className={styles.ruleTitle}>
								<strong>Rule {ruleIndex + 1}</strong>
								<small>Drop when all of these match</small>
							</div>
							<div className={styles.ruleActions}>
								<FilterDialog
									buttonText="Add filter"
									buttonIcon={null}
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
								<button
									type="button"
									className="secondary outline"
									aria-label="Remove rule"
									onClick={() => setRules(rules.filter((_, i) => i !== ruleIndex))}
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
											{filter.filterType !== "is_null" && <code>{ingestFilterValueLabel(filter)}</code>}
										</div>
										<button
											type="button"
											className="secondary outline"
											onClick={() => {
												const next = [...rules];
												next[ruleIndex] = {
													filters: rule.filters.filter((_, i) => i !== filterIndex),
												};
												setRules(next);
											}}
										>
											Remove
										</button>
									</div>
								))}
							</div>
						)}
					</article>
				))}
			</div>
		)}
	</section>
);
