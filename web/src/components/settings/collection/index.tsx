import styles from "./collection.module.css";

import { useEffect, useState } from "react";

import { api } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import type { CollectionSettings, DataRetention, GeoDetail, VisitorGroupMode } from "@/constants";
import { DocsLink, FiltersEditor, GeoSelect, VisitorModeSelect } from "../filters";
import {
	SettingsField,
	SettingsFieldset,
	SettingsForm,
	SettingsHeader,
	SettingsPanel,
	SettingsSwitch,
	SettingsTabs,
} from "../form";

type CollectionTab = (typeof collectionTabs)[number];

const formatCount = new Intl.NumberFormat().format;
const title = (value: string) => value.replaceAll("_", " ").replace(/\b\w/g, (char) => char.toUpperCase());

const collectionTabs = ["tracking", "filters", "retention"] as const;
const collectionTabItems = collectionTabs.map((value) => ({
	value,
	label: title(value),
}));

const retentionOptions = [
	{ value: "keep_all", label: "Keep all history" },
	{ value: "30", label: "1 month" },
	{ value: "90", label: "3 months" },
	{ value: "180", label: "6 months" },
	{ value: "365", label: "1 year" },
	{ value: "730", label: "2 years" },
] as const;
const retentionValues = retentionOptions.map((option) => option.value);

const retentionValue = (retention: DataRetention) => {
	if (retention.mode === "all" || retention.mode === "inherit") return "keep_all";
	const value = String(retention.days);
	return (retentionValues as readonly string[]).includes(value) ? value : "365";
};

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
	const [dataRetention, setDataRetention] = useState<DataRetention>({
		mode: "all",
	});
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
				setDataRetention(settings.dataRetention);
			})
			.catch((err) => setError(err.message));
	}, []);

	const saveSettings = (next: CollectionSettings) => {
		setSettings(next);
		api["/api/dashboard/settings"]
			.put({ json: next })
			.then(() => createToast("Collection settings updated", "success"))
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update collection settings");
				createToast("Failed to update collection settings", "error");
			});
	};

	const prune = (dryRun: boolean) => {
		setPruneError(undefined);
		api["/api/dashboard/settings/prune"]
			.post({ json: { dryRun } })
			.json()
			.then((result) => {
				if (typeof result === "string") throw new Error(result);
				setPruneResult(
					`${dryRun ? "Would delete" : "Deleted"} ${formatCount(result.total.deletedEvents)} of ${formatCount(result.total.totalEvents)} events.`,
				);
				setPruneResultOpen(true);
			})
			.catch((err) => {
				setPruneError(err instanceof Error ? err.message : "Failed to prune data");
			});
	};

	if (error) return <article role="alert">{error}</article>;
	if (!settings) return <div className="loading-spinner" />;

	return (
		<div className={styles.page}>
			<SettingsHeader
				title="Collection"
				description={
					<>
						Global defaults for collection and retention. Entity settings can override these per source. See{" "}
						<a href="https://liwan.dev/guides/cookie-banners/" target="_blank" rel="noopener noreferrer">
							cookie banner considerations
						</a>
						.
					</>
				}
			/>
			<SettingsForm id="collection-settings-form">
				<SettingsTabs value={tab} onValueChange={setTab} tabs={collectionTabItems}>
					<SettingsPanel value="tracking">
						<SettingsField
							label="Visitor grouping"
							description={
								<>
									Controls how repeat visits are grouped without storing raw IP addresses.{" "}
									<DocsLink hash="visitor-grouping" />
								</>
							}
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
							description={
								<>
									Choose how much location data to store for new events. <DocsLink hash="geolocation" />
								</>
							}
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
							description={
								<>
									Required for bounce rate, time on site, entry page, and exit page. <DocsLink hash="session-metrics" />
								</>
							}
							checked={trackSessions}
							onCheckedChange={(checked) => {
								setTrackSessions(checked);
								saveSettings({ ...settings, trackSessions: checked });
							}}
						/>
						<SettingsSwitch
							name="trackUtmParams"
							label="Track UTM parameters"
							description={
								<>
									Stores campaign fields like source, medium, campaign, term, and content.{" "}
									<DocsLink hash="utm-parameters" />
								</>
							}
							checked={trackUtmParams}
							onCheckedChange={(checked) => {
								setTrackUtmParams(checked);
								saveSettings({ ...settings, trackUtmParams: checked });
							}}
						/>
					</SettingsPanel>
					<SettingsPanel value="filters">
						<FiltersEditor
							rules={settings.ingestDropRules}
							setRules={(ingestDropRules) => saveSettings({ ...settings, ingestDropRules })}
						/>
					</SettingsPanel>
					<SettingsPanel value="retention">
						<SettingsField
							label="History retention"
							description={
								<>
									Automatically delete event data older than the selected period.{" "}
									<DocsLink hash="retention-and-pruning" />
								</>
							}
							name="historyRetention"
						>
							<select
								name="historyRetention"
								value={retentionValue(dataRetention)}
								onChange={(event) => {
									const next = event.currentTarget.value;
									if (!(retentionValues as readonly string[]).includes(next)) return;
									if (next === "keep_all") {
										const dataRetention = { mode: "all" } as const;
										setDataRetention(dataRetention);
										saveSettings({ ...settings, dataRetention });
									} else {
										const dataRetention = {
											mode: "days",
											days: Number(next),
										} as const;
										setDataRetention(dataRetention);
										saveSettings({ ...settings, dataRetention });
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
							legend="Prune data"
							description={
								<>
									Pruning applies saved retention, UTM, geolocation, and session settings to historical data. Drop rules
									only affect new events. Settings save automatically; run a dry run first to preview changes.{" "}
									<DocsLink hash="retention-and-pruning" />
								</>
							}
						>
							<div className={styles.pruneActions}>
								<button type="button" className="secondary outline" onClick={() => prune(true)}>
									Dry run
								</button>
								<Dialog
									title="Prune data?"
									description="This permanently applies the current collection settings to historical data. Run a dry run first to preview the changes."
									trigger={<button type="button">Prune now</button>}
								>
									<div className="grid">
										<Dialog.Close className="secondary outline">Cancel</Dialog.Close>
										<Dialog.Close onClick={() => prune(false)}>Prune now</Dialog.Close>
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
