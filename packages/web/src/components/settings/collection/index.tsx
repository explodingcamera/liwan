import styles from "./collection.module.css";

import { useState } from "react";

import { api, queryClient, useQuery } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { createToast } from "@/components/ui/toast";
import type { CollectionSettings, DataRetention } from "@/constants";
import { DocsLink, FiltersEditor, GeoSelect, VisitorModeSelect } from "../filters";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsPanel, SettingsSwitch, SettingsTabs } from "../form";

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
	const { data: settings, error: loadError } = useQuery({
		queryKey: ["collection-settings"],
		staleTime: 30_000,
		queryFn: () => api["/api/dashboard/settings"].get().json(),
	});
	const [saveError, setSaveError] = useState<string>();
	const [tab, setTab] = useState<CollectionTab>("tracking");
	const [pruneResult, setPruneResult] = useState<string>();
	const [pruneResultOpen, setPruneResultOpen] = useState(false);
	const [pruneError, setPruneError] = useState<string>();

	const saveSettings = (next: CollectionSettings) => {
		setSaveError(undefined);
		queryClient.setQueryData(["collection-settings"], next);
		api["/api/dashboard/settings"]
			.put({ json: next })
			.then(() => createToast("Collection settings updated", "success"))
			.catch((err) => {
				setSaveError(err instanceof Error ? err.message : "Failed to update collection settings");
				queryClient.invalidateQueries({ queryKey: ["collection-settings"] });
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
				const message = `${dryRun ? "Would delete" : "Deleted"} ${formatCount(result.total.deletedEvents)} of ${formatCount(result.total.totalEvents)} events.`;
				if (dryRun) {
					createToast(message, "info");
					return;
				}
				setPruneResult(message);
				setPruneResultOpen(true);
			})
			.catch((err) => {
				setPruneError(err instanceof Error ? err.message : "Failed to prune data");
			});
	};

	const error =
		saveError ?? (loadError ? (loadError instanceof Error ? loadError.message : "Failed to load settings") : undefined);

	return (
		<div className={styles.page}>
			{error && <article role="alert">{error}</article>}
			{!settings && !loadError && <LoadingSpinner />}
			{settings && (
				<SettingsForm id="collection-settings-form">
					<SettingsTabs value={tab} onValueChange={setTab} tabs={collectionTabItems}>
						<SettingsPanel value="tracking">
							<SettingsField
								label="Visitor grouping"
								description={
									<>
										Group repeat visits without storing raw IP addresses. <DocsLink hash="visitor-grouping" />
									</>
								}
								name="visitorGroupMode"
							>
								<VisitorModeSelect
									id="visitorGroupMode"
									value={settings.visitorGroupMode}
									onChange={(value) => {
										if (!value) return;
										saveSettings({ ...settings, visitorGroupMode: value });
									}}
								/>
							</SettingsField>
							<SettingsField
								label="Geolocation detail"
								description={
									<>
										Choose the location detail stored for new events. <DocsLink hash="geolocation" />
									</>
								}
								name="trackGeo"
							>
								<GeoSelect
									id="trackGeo"
									value={settings.trackGeo}
									onChange={(value) => {
										if (!value) return;
										saveSettings({ ...settings, trackGeo: value });
									}}
								/>
							</SettingsField>
							<SettingsSwitch
								name="trackSessions"
								label="Track session metrics"
								description={
									<>
										Collect bounce rate, time on site, and entry and exit pages. <DocsLink hash="session-metrics" />
									</>
								}
								checked={settings.trackSessions}
								onCheckedChange={(checked) => {
									saveSettings({ ...settings, trackSessions: checked });
								}}
							/>
							<SettingsSwitch
								name="trackUtmParams"
								label="Track UTM parameters"
								description={
									<>
										Collect source, medium, campaign, term, and content. <DocsLink hash="utm-parameters" />
									</>
								}
								checked={settings.trackUtmParams}
								onCheckedChange={(checked) => {
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
									value={retentionValue(settings.dataRetention)}
									onChange={(event) => {
										const next = event.currentTarget.value;
										if (!(retentionValues as readonly string[]).includes(next)) return;
										if (next === "keep_all") {
											const dataRetention = { mode: "all" } as const;
											saveSettings({ ...settings, dataRetention });
										} else {
											const dataRetention = {
												mode: "days",
												days: Number(next),
											} as const;
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
										Apply saved collection and retention settings to existing events. Drop rules only affect new events.
										Run a dry run to preview changes. <DocsLink hash="retention-and-pruning" />
									</>
								}
							>
								<div className={styles.pruneActions}>
									<button type="button" className="button-secondary" onClick={() => prune(true)}>
										Dry run
									</button>
									<Dialog
										title="Prune data?"
										description="This permanently applies the current collection settings to historical data. Run a dry run first to preview the changes."
										trigger={
											<button type="button" className="button-primary">
												Prune now
											</button>
										}
									>
										<div className="action-row">
											<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
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
			)}
		</div>
	);
};
