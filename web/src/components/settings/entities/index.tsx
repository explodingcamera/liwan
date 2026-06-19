import styles from "../settings.module.css";

import { Fragment, useEffect, useMemo, useState } from "react";
import { SettingsIcon } from "lucide-react";

import { api } from "@/api";
import { Snippet } from "@/components/ui/snippet";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import type { EntityCollectionSettings } from "@/constants";
import { invalidateEntities, useEntities, useMe, useProjects } from "@/hooks/api";
import { DeleteDialog } from "../dialogs";
import { AllowedHostnamesEditor, DocsLink, FiltersEditor, GeoSelect, VisitorModeSelect } from "../filters";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsHeader, SettingsPanel, SettingsTabs } from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export { CreateEntity } from "./dialogs";

type EntityTab = "general" | "collection" | "filters";
type EntitySettingsSection = "collection" | "filters";

const retentionOptions = [
	{ value: "inherit", label: "Inherit global" },
	{ value: "keep_all", label: "Keep all history" },
	{ value: "30", label: "1 month" },
	{ value: "90", label: "3 months" },
	{ value: "180", label: "6 months" },
	{ value: "365", label: "1 year" },
	{ value: "730", label: "2 years" },
] as const;
const retentionValues = retentionOptions.map((option) => option.value);

const retentionValue = (retention: EntityCollectionSettings["dataRetention"]) => {
	if (retention.mode === "inherit") return "inherit";
	if (retention.mode === "all") return "keep_all";
	const value = String(retention.days);
	return (retentionValues as readonly string[]).includes(value) ? value : "365";
};

const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};

const entityTabs = [
	{ value: "general", label: "General" },
	{ value: "collection", label: "Collection" },
	{ value: "filters", label: "Filters" },
] as const satisfies readonly { value: EntityTab; label: string }[];

const SettingsLink = ({ href, label }: { href: string; label: string }) => {
	const { role } = useMe();
	if (role === "user") return null;

	return (
		<a href={href} className={styles.settingsLink} aria-label={label} title={label}>
			<SettingsIcon size={18} />
		</a>
	);
};

const EntityId = ({ id }: { id: string }) => (
	<button
		type="button"
		className={styles.entityId}
		onClick={() =>
			navigator.clipboard
				.writeText(id)
				.then(() => createToast("Entity ID copied to clipboard", "info"))
				.catch(() => {})
		}
	>
		{id}
	</button>
);

export const EntitiesTable = () => {
	const { entities, isLoading, authError } = useEntities();

	if (authError) {
		return "You don't have permission to view this page.";
	}

	const columns: Column<(typeof entities)[number]>[] = [
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={`/settings/entities/${row.id}`}>{row.displayName}</a>,
			nowrap: true,
		},
		{
			id: "id",
			header: "ID",
			render: (row) => <EntityId id={row.id} />,
			nowrap: true,
		},
		{
			id: "projects",
			header: "Projects",
			render: (row) => (
				<>
					{row.projects.map((project, i) => (
						<Fragment key={project.id}>
							{i > 0 && ", "}
							<a href={`/settings/projects/${project.id}`}>{project.displayName}</a>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={`/settings/entities/${row.id}`} label={`Open ${row.displayName} settings`} />
			),
		},
	];

	return <Table columns={columns} rows={entities} isLoading={isLoading} />;
};

export const EntitySettingsPage = ({ entityId }: { entityId: string }) => {
	const [resolvedEntityId, setResolvedEntityId] = useState<string>();

	useEffect(() => {
		setResolvedEntityId(getSettingsPathId("/settings/entities/") || entityId);
	}, [entityId]);

	if (!resolvedEntityId) return <div className="loading-spinner" />;
	return <EntitySettingsContent entityId={resolvedEntityId} />;
};

const EntitySettingsContent = ({ entityId }: { entityId: string }) => {
	const { entities, isLoading, authError } = useEntities();
	const { projects } = useProjects();
	const entity = entities.find((entity) => entity.id === entityId);
	const [tab, setTab] = useState<EntityTab>("general");
	const [displayName, setDisplayName] = useState("");
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);
	const [settings, setSettings] = useState<EntityCollectionSettings>();
	const [error, setError] = useState<string>();

	const projectTags = useMemo(
		() =>
			projects.map((project) => ({
				value: project.id,
				label: project.displayName,
			})),
		[projects],
	);

	useEffect(() => {
		if (!entity) return;
		setDisplayName(entity.displayName);
		setSelectedProjects(
			entity.projects.map((project) => ({
				value: project.id,
				label: project.displayName,
			})),
		);
		api["/api/dashboard/entity/{entity_id}/settings"]
			.get({ params: { entity_id: entity.id } })
			.json()
			.then((res) => setSettings(res.settings))
			.catch((err) => setError(err instanceof Error ? err.message : "Failed to load entity settings"));
	}, [entity]);

	const saveEntity = (nextDisplayName: string, nextProjects: Tag[]) => {
		if (!entity) return;
		api["/api/dashboard/entity/{entity_id}"]
			.put({
				params: { entity_id: entity.id },
				json: {
					displayName: nextDisplayName,
					projects: nextProjects.map((tag) => String(tag.value)),
				},
			})
			.then(() => {
				invalidateEntities();
				createToast("Entity updated", "success");
			})
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update entity");
				createToast("Failed to update entity", "error");
			});
	};

	const saveEntitySettings = (next: EntityCollectionSettings, section: EntitySettingsSection = "collection") => {
		if (!entity) return;
		const label = section === "filters" ? "filters" : "collection";
		setSettings(next);
		api["/api/dashboard/entity/{entity_id}/settings"]
			.put({
				params: { entity_id: entity.id },
				json: next,
			})
			.then(() => createToast(`Entity ${label} updated`, "success"))
			.catch((err) => {
				setError(err instanceof Error ? err.message : `Failed to update entity ${label} settings`);
				createToast(`Failed to update entity ${label}`, "error");
			});
	};

	const saveCollectionSettings = (
		patch: Partial<EntityCollectionSettings>,
		section: EntitySettingsSection = "collection",
	) => {
		if (!entity || !settings) return;
		saveEntitySettings({ ...settings, entityId: entity.id, ...patch }, section);
	};

	if (authError) return <p>You don't have permission to view this page.</p>;
	if (isLoading) return <div className="loading-spinner" />;
	if (!entity) return <p>Entity not found.</p>;

	return (
		<SettingsForm>
			<SettingsHeader
				title={displayName || entity.displayName}
				backHref="/settings/entities"
				backLabel="Back to entities"
			/>
			<SettingsTabs value={tab} onValueChange={setTab} tabs={entityTabs}>
				<SettingsPanel value="general">
					<SettingsField label="Entity name" name="displayName">
						<input
							required
							name="displayName"
							type="text"
							value={displayName}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
							onBlur={(event) => {
								if (event.currentTarget.value !== entity.displayName) {
									saveEntity(event.currentTarget.value, selectedProjects);
								}
							}}
							autoComplete="off"
						/>
					</SettingsField>
					<SettingsFieldset
						legend="Tracking snippet"
						description="Add this script to pages that should send events for this entity."
					>
						<Snippet entityId={entity.id} />
					</SettingsFieldset>
					<Tags
						labelText="Associated projects"
						labelDescription="Controls which projects include data from this entity."
						selected={selectedProjects}
						suggestions={projectTags}
						onAdd={(tag) => {
							const next = [...selectedProjects, tag];
							setSelectedProjects(next);
							saveEntity(displayName, next);
						}}
						onDelete={(i) => {
							const next = selectedProjects.filter((_, index) => index !== i);
							setSelectedProjects(next);
							saveEntity(displayName, next);
						}}
						noOptionsText="No matching projects"
					/>
					<div className={styles.dangerZone}>
						<DeleteDialog
							id={entity.id}
							displayName={entity.displayName}
							type="entity"
							onDeleted={() => {
								window.location.href = "/settings/entities";
							}}
							trigger={
								<button type="button" className={styles.deleteButton}>
									Delete entity
								</button>
							}
						/>
					</div>
				</SettingsPanel>
				{settings && (
					<>
						<SettingsPanel value="collection">
							<SettingsField
								label="Visitor grouping"
								description={
									<>
										Controls how repeat visits are grouped for this entity. <DocsLink hash="visitor-grouping" />
									</>
								}
								name="visitorGroupMode"
							>
								<VisitorModeSelect
									id="entityVisitorGroupMode"
									value={settings.visitorGroupMode ?? null}
									onChange={(visitorGroupMode) => saveCollectionSettings({ visitorGroupMode })}
									allowInherit
								/>
							</SettingsField>
							<SettingsField
								label="Session metrics"
								description={
									<>
										Required for bounce rate, time on site, entry page, and exit page.{" "}
										<DocsLink hash="session-metrics" />
									</>
								}
								name="trackSessions"
							>
								<select
									name="trackSessions"
									value={settings.trackSessions == null ? "inherit" : String(settings.trackSessions)}
									onChange={(event) => {
										const trackSessions =
											event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true";
										saveCollectionSettings({ trackSessions });
									}}
								>
									<option value="inherit">Inherit global</option>
									<option value="true">Track</option>
									<option value="false">Do not track</option>
								</select>
							</SettingsField>
							<SettingsField
								label="UTM parameters"
								description={
									<>
										Stores campaign fields like source, medium, campaign, term, and content.{" "}
										<DocsLink hash="utm-parameters" />
									</>
								}
								name="trackUtmParams"
							>
								<select
									name="trackUtmParams"
									value={settings.trackUtmParams == null ? "inherit" : String(settings.trackUtmParams)}
									onChange={(event) => {
										const trackUtmParams =
											event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true";
										saveCollectionSettings({ trackUtmParams });
									}}
								>
									<option value="inherit">Inherit global</option>
									<option value="true">Track</option>
									<option value="false">Do not track</option>
								</select>
							</SettingsField>
							<SettingsField
								label="Geolocation detail"
								description={
									<>
										Choose how much location data to store for this entity. <DocsLink hash="geolocation" />
									</>
								}
								name="trackGeo"
							>
								<GeoSelect
									id="entityTrackGeo"
									value={settings.trackGeo ?? null}
									onChange={(trackGeo) => saveCollectionSettings({ trackGeo })}
									allowInherit
								/>
							</SettingsField>
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
										if (next === "inherit") {
											saveCollectionSettings({
												dataRetention: { mode: "inherit" },
											});
										} else if (next === "keep_all") {
											saveCollectionSettings({
												dataRetention: { mode: "all" },
											});
										} else {
											saveCollectionSettings({
												dataRetention: { mode: "days", days: Number(next) },
											});
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
						</SettingsPanel>
						<SettingsPanel value="filters">
							<SettingsField label="Allowed hostnames" name="allowedHostnames">
								<AllowedHostnamesEditor
									value={settings.allowedHostnames}
									onChange={(allowedHostnames) => saveCollectionSettings({ allowedHostnames }, "filters")}
								/>
							</SettingsField>
							<FiltersEditor
								rules={settings.ingestDropRules}
								setRules={(ingestDropRules) => saveCollectionSettings({ ingestDropRules }, "filters")}
								scope="entity"
							/>
						</SettingsPanel>
					</>
				)}
			</SettingsTabs>
			{error && (
				<article role="alert" className={styles.error}>
					{error}
				</article>
			)}
		</SettingsForm>
	);
};
