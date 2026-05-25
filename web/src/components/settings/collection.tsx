import { useEffect, useState } from "react";
import type { OASModel } from "fets";
import { createToast } from "../toast";
import type { DashboardSpec } from "../../api";
import { api, dimensions } from "../../api";
import { Dialog } from "../dialog";
import { FilterDialog, filterOptions, type FilterOption, type GenericFilter } from "../project/filter";
import styles from "./collection.module.css";
import {
	SettingsField,
	SettingsFieldset,
	SettingsForm,
	SettingsHeader,
	SettingsPanel,
	SettingsSwitch,
	SettingsTabs,
} from "./form";

type CollectionSettings = OASModel<DashboardSpec, "CollectionSettings">;
type IngestFilter = OASModel<DashboardSpec, "IngestFilter">;
type VisitorGroupMode = CollectionSettings["visitorGroupMode"];
type GeoDetail = CollectionSettings["trackGeo"];
type HistoryMode = CollectionSettings["historyMode"];
type CollectionTab = "tracking" | "filters" | "purging";

const title = (value: string) => value.replaceAll("_", " ").replace(/\b\w/g, (char) => char.toUpperCase());
const formatCount = new Intl.NumberFormat().format;
const ingestDimensions = ["event", ...dimensions] as const;

const isOneOf = <T extends string>(values: readonly T[], value: string): value is T =>
	values.some((item) => item === value);

const visitorGroupModes = [
	"accurate",
	"random_per_request",
	"network_standard",
	"network_balanced",
	"network_accurate",
] as const satisfies readonly VisitorGroupMode[];
const geoDetails = ["none", "country", "city"] as const satisfies readonly GeoDetail[];
const retentionOptions = [
	{ value: "keep_all", label: "Keep all history" },
	{ value: "30", label: "1 month" },
	{ value: "90", label: "3 months" },
	{ value: "180", label: "6 months" },
	{ value: "365", label: "1 year" },
	{ value: "730", label: "2 years" },
] as const;
const retentionValues = retentionOptions.map((option) => option.value);
const retentionValue = (mode: HistoryMode, days?: number | null) => {
	if (mode === "keep_all") return "keep_all";
	const value = String(days ?? 365);
	return isOneOf(retentionValues, value) ? value : "365";
};
const collectionTabs = ["tracking", "filters", "purging"] as const satisfies readonly CollectionTab[];
const collectionTabItems = collectionTabs.map((value) => ({ value, label: title(value) }));
const ingestFilterOptions: Record<string, FilterOption> = {
	event: {
		label: "Event",
		invertable: false,
		filterTypes: ["equal", "contains", "starts_with", "ends_with", "is_null"],
	},
	...filterOptions,
};

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
			if (isOneOf(visitorGroupModes, next)) onChange?.(next);
		}}
	>
		{allowInherit && <option value="inherit">Inherit global</option>}
		<option value="accurate">Accurate</option>
		<option value="random_per_request">Random per request</option>
		<option value="network_standard">Network standard (/24 IPv4)</option>
		<option value="network_balanced">Network balanced (/28 IPv4)</option>
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
			if (isOneOf(geoDetails, next)) onChange?.(next);
		}}
	>
		{allowInherit && <option value="inherit">Inherit global</option>}
		<option value="none">No geolocation lookup</option>
		<option value="country">Country only</option>
		<option value="city">Country and city</option>
	</select>
);

export const FiltersEditor = ({
	filters,
	setFilters,
}: {
	filters: IngestFilter[];
	setFilters: (filters: IngestFilter[]) => void;
}) => (
	<section className={styles.section}>
		<div className={styles.sectionHeader}>
			<h2 className={styles.sectionTitle}>Ingest filters</h2>
			<div className={styles.filterAction}>
				<FilterDialog
					buttonText="+"
					buttonIcon={null}
					dimensions={[...ingestDimensions]}
					options={ingestFilterOptions}
					allowInverted={false}
					onAdd={(filter: GenericFilter) =>
						setFilters([
							...filters,
							{ dimension: filter.dimension, filterType: filter.filterType, value: filter.value, action: "drop" },
						])
					}
				/>
			</div>
		</div>
		<p>Matching events are dropped before they are stored.</p>
		{filters.length === 0 ? (
			<small>No ingest filters right now.</small>
		) : (
			<div className={styles.filterList}>
				{filters.map((filter, index) => (
					<div className={styles.filterRow} key={`${filter.dimension}-${index}`}>
						<div className={styles.filterText}>
							<strong>{title(filter.dimension)}</strong>
							<span>{title(filter.filterType)}</span>
							{filter.value && <code>{filter.value}</code>}
						</div>
						<button
							type="button"
							className="secondary outline"
							onClick={() => setFilters(filters.filter((_, i) => i !== index))}
						>
							Remove
						</button>
					</div>
				))}
			</div>
		)}
	</section>
);

export const CollectionSettingsPage = () => {
	const [settings, setSettings] = useState<CollectionSettings>();
	const [error, setError] = useState<string>();
	const [tab, setTab] = useState<CollectionTab>("tracking");
	const [pruneResult, setPruneResult] = useState<string>();
	const [pruneResultOpen, setPruneResultOpen] = useState(false);
	const [visitorGroupMode, setVisitorGroupMode] = useState<VisitorGroupMode>("accurate");
	const [trackSessions, setTrackSessions] = useState(true);
	const [trackUtmParams, setTrackUtmParams] = useState(true);
	const [trackGeo, setTrackGeo] = useState<GeoDetail>("city");
	const [historyMode, setHistoryMode] = useState<HistoryMode>("keep_all");
	const [historyDays, setHistoryDays] = useState<number | null>(null);
	const [pruneError, setPruneError] = useState<string>();

	useEffect(() => {
		api["/api/dashboard/settings"]
			.get()
			.json()
			.then((settings) => {
				setSettings(settings);
				setVisitorGroupMode(settings.visitorGroupMode);
				setTrackSessions(settings.trackSessions);
				setTrackUtmParams(settings.trackUtmParams);
				setTrackGeo(settings.trackGeo);
				setHistoryMode(settings.historyMode);
				setHistoryDays(settings.historyMode === "days" ? (settings.historyDays ?? 365) : null);
			})
			.catch((err) => setError(err.message));
	}, []);

	const saveSettings = async (next: CollectionSettings) => {
		setSettings(next);
		try {
			await api["/api/dashboard/settings"].put({ json: next });
			createToast("Collection settings updated", "success");
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to update collection settings");
			createToast("Failed to update collection settings", "error");
		}
	};

	const prune = async (dryRun: boolean) => {
		try {
			setPruneError(undefined);
			const result = await api["/api/dashboard/settings/prune"].post({ json: { dryRun } }).json();
			if (typeof result === "string") throw new Error(result);
			setPruneResult(
				`${dryRun ? "Would delete" : "Deleted"} ${formatCount(result.total.deletedEvents)} of ${formatCount(result.total.totalEvents)} events.`,
			);
			setPruneResultOpen(true);
		} catch (err) {
			setPruneError(err instanceof Error ? err.message : "Failed to prune data");
		}
	};

	if (error) return <article role="alert">{error}</article>;
	if (!settings) return <div className="loading-spinner" />;

	return (
		<div className={styles.page}>
			<SettingsHeader
				title="Collection"
				description="Global defaults for collection and retention. Entity settings can override these per source."
			/>
			<SettingsForm id="collection-settings-form">
				<SettingsTabs value={tab} onValueChange={setTab} tabs={collectionTabItems}>
					<SettingsPanel value="tracking">
						<SettingsField
							label="Visitor grouping"
							description="Controls how repeat visits are grouped without storing raw IP addresses."
							name="visitorGroupMode"
						>
							<VisitorModeSelect
								id="visitorGroupMode"
								value={visitorGroupMode}
								onChange={(value) => {
									if (!value) return;
									setVisitorGroupMode(value);
									saveSettings({ ...settings, visitorGroupMode: value });
								}}
							/>
						</SettingsField>
						<SettingsField
							label="Geolocation detail"
							description="Choose how much location data is stored for new events."
							name="trackGeo"
						>
							<GeoSelect
								id="trackGeo"
								value={trackGeo}
								onChange={(value) => {
									if (!value) return;
									setTrackGeo(value);
									saveSettings({ ...settings, trackGeo: value });
								}}
							/>
						</SettingsField>
						<SettingsSwitch
							name="trackSessions"
							label="Track session metrics"
							description="Required for bounce rate, time on site, entry URL, and exit URL."
							checked={trackSessions}
							onCheckedChange={(checked) => {
								setTrackSessions(checked);
								saveSettings({ ...settings, trackSessions: checked });
							}}
						/>
						<SettingsSwitch
							name="trackUtmParams"
							label="Track UTM parameters"
							description="Stores campaign fields like source, medium, campaign, term, and content."
							checked={trackUtmParams}
							onCheckedChange={(checked) => {
								setTrackUtmParams(checked);
								saveSettings({ ...settings, trackUtmParams: checked });
							}}
						/>
					</SettingsPanel>
					<SettingsPanel value="filters">
						<FiltersEditor
							filters={settings.ingestFilters}
							setFilters={(ingestFilters) => saveSettings({ ...settings, ingestFilters })}
						/>
					</SettingsPanel>
					<SettingsPanel value="purging">
						<SettingsField
							label="History retention"
							description="Automatically prune older event data after the selected period."
							name="historyRetention"
						>
							<select
								name="historyRetention"
								value={retentionValue(historyMode, historyDays)}
								onChange={(event) => {
									const next = event.currentTarget.value;
									if (!isOneOf(retentionValues, next)) return;
									if (next === "keep_all") {
										setHistoryMode("keep_all");
										setHistoryDays(null);
										saveSettings({ ...settings, historyMode: "keep_all", historyDays: null });
									} else {
										const historyDays = Number(next);
										setHistoryMode("days");
										setHistoryDays(historyDays);
										saveSettings({ ...settings, historyMode: "days", historyDays });
									}
								}}
							>
								{retentionOptions.map((option) => (
									<option key={option.value} value={option.value}>
										{option.label}
									</option>
								))}
							</select>
						</SettingsField>
						<SettingsFieldset
							legend="Prune Data"
							description="Pruning applies saved retention, UTM, geolocation, and session settings to historical data. Ingest filters only affect new events. Settings save automatically; run a dry run to preview."
						>
							<div className={styles.pruneActions}>
								<button type="button" className="secondary outline" onClick={() => prune(true)}>
									Dry Run
								</button>
								<Dialog
									title="Prune data?"
									description="This permanently applies the current collection settings to historical data. Run a dry run first if you want to preview the changes."
									trigger={<button type="button">Prune Now</button>}
								>
									<div className="grid">
										<Dialog.Close className="secondary outline">Cancel</Dialog.Close>
										<Dialog.Close onClick={() => prune(false)}>Prune Now</Dialog.Close>
									</div>
								</Dialog>
							</div>
							{pruneError && <article role="alert">{pruneError}</article>}
						</SettingsFieldset>
						<Dialog title="Prune result" open={pruneResultOpen} onOpenChange={setPruneResultOpen} trigger={false}>
							<p>{pruneResult}</p>
							<Dialog.Close>Close</Dialog.Close>
						</Dialog>
					</SettingsPanel>
				</SettingsTabs>
			</SettingsForm>
		</div>
	);
};
