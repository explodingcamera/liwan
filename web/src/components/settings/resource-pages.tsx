import styles from "./resource-pages.module.css";

import { useEffect, useMemo, useState } from "react";
import { navigate } from "astro:transitions/client";
import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";

import { api } from "../../api";
import type {
	DataRetention,
	Dimension,
	DisplayOverride,
	EntityCollectionSettings,
	GeoDetail,
	Metric,
	ProjectDisplaySettings,
	VisitorGroupMode,
} from "../../constants";
import { dimensionNames, displayOverrides, metricNames, metrics } from "../../constants";
import { invalidateEntities, invalidateProjects, useEntities, useProjects } from "../../hooks/api";
import type { Tag } from "../tags";
import { Tags } from "../tags";
import { createToast } from "../toast";
import { DocsLink, FiltersEditor, GeoSelect, VisitorModeSelect } from "./collection";
import { DeleteDialog } from "./dialogs";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsHeader, SettingsPanel, SettingsTabs } from "./form";
import { Snippet } from "./snippet";

type ProjectTab = "general" | "display";
type EntityTab = "general" | "collection" | "filters";

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

const retentionValue = (retention: DataRetention) => {
	if (retention.mode === "inherit") return "inherit";
	if (retention.mode === "all") return "keep_all";
	const value = String(retention.days);
	return (retentionValues as readonly string[]).includes(value) ? value : "365";
};
const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};

const displayLabels: Record<DisplayOverride, string> = {
	auto: "Auto",
	show: "Always",
	hide: "Hidden",
};
const displayDimensionGroups = [
	{ label: "Pages", dimensions: ["url", "url_entry", "url_exit", "fqdn"] },
	{
		label: "Campaigns",
		dimensions: ["referrer", "utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"],
	},
	{ label: "Geography", dimensions: ["country", "city"] },
	{ label: "Technology", dimensions: ["platform", "browser"] },
	{ label: "Device", dimensions: ["mobile", "screen_width", "orientation"] },
] as const satisfies readonly { label: string; dimensions: readonly Dimension[] }[];
const projectTabs = [
	{ value: "general", label: "General" },
	{ value: "display", label: "Display" },
] as const satisfies readonly { value: ProjectTab; label: string }[];
const entityTabs = [
	{ value: "general", label: "General" },
	{ value: "collection", label: "Collection" },
	{ value: "filters", label: "Filters" },
] as const satisfies readonly { value: EntityTab; label: string }[];

const DisplayOverrideSwitch = ({
	label,
	value,
	onChange,
}: {
	label: string;
	value: DisplayOverride;
	onChange: (value: DisplayOverride) => void;
}) => (
	<div className={styles.displayRow}>
		<span>{label}</span>
		<ToggleGroup
			className={styles.segmented}
			aria-label={`${label} display`}
			value={[value]}
			onValueChange={(next) => {
				const [selected] = next;
				if (selected) onChange(selected);
			}}
		>
			{displayOverrides.map((option) => (
				<Toggle key={option} value={option} aria-label={`${displayLabels[option]} ${label}`}>
					{displayLabels[option]}
				</Toggle>
			))}
		</ToggleGroup>
	</div>
);

export const ProjectSettingsPage = ({ projectId }: { projectId: string }) => {
	const [resolvedProjectId, setResolvedProjectId] = useState<string>();

	useEffect(() => {
		setResolvedProjectId(getSettingsPathId("/settings/projects/") || projectId);
	}, [projectId]);

	if (!resolvedProjectId) return <div className="loading-spinner" />;
	return <ProjectSettingsContent projectId={resolvedProjectId} />;
};

const ProjectSettingsContent = ({ projectId }: { projectId: string }) => {
	const { projects, isLoading } = useProjects();
	const { entities } = useEntities();
	const project = projects.find((project) => project.id === projectId);
	const [tab, setTab] = useState<ProjectTab>("general");
	const [displayName, setDisplayName] = useState("");
	const [visibility, setVisibility] = useState<"private" | "public">("private");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [settings, setSettings] = useState<ProjectDisplaySettings>();
	const [error, setError] = useState<string>();

	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);

	useEffect(() => {
		if (!project) return;
		const entities = project.entities.map((entity) => ({ value: entity.id, label: entity.displayName }));
		setDisplayName(project.displayName);
		setVisibility(project.public ? "public" : "private");
		setSelectedEntities(entities);
		api["/api/dashboard/project/{project_id}/settings"]
			.get({ params: { project_id: project.id } })
			.json()
			.then(setSettings)
			.catch((err) => setError(err instanceof Error ? err.message : "Failed to load project settings"));
	}, [project]);

	const saveProject = (nextDisplayName: string, nextVisibility: "private" | "public", nextEntities: Tag[]) => {
		if (!project) return;
		api["/api/dashboard/project/{project_id}"]
			.put({
				params: { project_id: project.id },
				json: {
					project: { displayName: nextDisplayName, public: nextVisibility === "public" },
					entities: nextEntities.map((tag) => String(tag.value)),
				},
			})
			.then(() => {
				invalidateProjects();
				createToast("Project updated", "success");
			})
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update project");
				createToast("Failed to update project", "error");
			});
	};

	const saveProjectSettings = (next: ProjectDisplaySettings) => {
		if (!project) return;
		setSettings(next);
		api["/api/dashboard/project/{project_id}/settings"]
			.put({
				params: { project_id: project.id },
				json: next,
			})
			.then(() => {
				invalidateProjects();
				createToast("Project display updated", "success");
			})
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update project display settings");
				createToast("Failed to update project display", "error");
			});
	};

	const updateMetricDisplay = (metric: Metric, value: DisplayOverride) => {
		if (!settings) return;
		const metricDisplayOverrides = { ...settings.metricDisplayOverrides };
		if (value === "auto") delete metricDisplayOverrides[metric];
		else metricDisplayOverrides[metric] = value;
		saveProjectSettings({ ...settings, metricDisplayOverrides });
	};

	const updateDimensionDisplay = (dimension: Dimension, value: DisplayOverride) => {
		if (!settings) return;
		const dimensionDisplayOverrides = { ...settings.dimensionDisplayOverrides };
		if (value === "auto") delete dimensionDisplayOverrides[dimension];
		else dimensionDisplayOverrides[dimension] = value;
		saveProjectSettings({ ...settings, dimensionDisplayOverrides });
	};

	if (isLoading) return <div className="loading-spinner" />;
	if (!project) return <p>Project not found.</p>;

	return (
		<SettingsForm>
			<SettingsHeader
				title={displayName || project.displayName}
				backHref="/settings/projects"
				backLabel="Back to projects"
			/>
			<SettingsTabs value={tab} onValueChange={setTab} tabs={projectTabs}>
				<SettingsPanel value="general">
					<SettingsField label="Project name" description="Used in the dashboard." name="displayName">
						<input
							required
							name="displayName"
							type="text"
							value={displayName}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
							onBlur={(event) => {
								if (event.currentTarget.value !== project.displayName) {
									saveProject(event.currentTarget.value, visibility, selectedEntities);
								}
							}}
							autoComplete="off"
						/>
					</SettingsField>
					<SettingsField
						label="Visibility"
						description="Public projects can be viewed by anyone, even if they are not logged in."
						name="visibility"
					>
						<select
							name="visibility"
							value={visibility}
							onChange={(event) => {
								const next = event.currentTarget.value === "public" ? "public" : "private";
								setVisibility(next);
								saveProject(displayName, next, selectedEntities);
							}}
						>
							<option value="private">Private</option>
							<option value="public">Public</option>
						</select>
					</SettingsField>
					<Tags
						labelText="Associated entities"
						selected={selectedEntities}
						suggestions={entityTags}
						onAdd={(tag) => {
							const next = [...selectedEntities, tag];
							setSelectedEntities(next);
							saveProject(displayName, visibility, next);
						}}
						onDelete={(i) => {
							const next = selectedEntities.filter((_, index) => index !== i);
							setSelectedEntities(next);
							saveProject(displayName, visibility, next);
						}}
						noOptionsText="No matching entities"
					/>
					<div className={styles.dangerZone}>
						<DeleteDialog
							id={project.id}
							displayName={project.displayName}
							type="project"
							onDeleted={() => navigate("/settings/projects")}
							trigger={
								<button type="button" className={styles.deleteButton}>
									Delete project
								</button>
							}
						/>
					</div>
				</SettingsPanel>
				{settings && (
					<SettingsPanel value="display">
						<p className={styles.displayHelp}>
							Auto uses Liwan's visibility rules. Always forces an item to show for this project. Hidden removes it from
							reports.
						</p>
						<div className={styles.displaySections}>
							<SettingsFieldset legend="Metrics">
								<div className={styles.displayGrid}>
									{metrics.map((metric) => (
										<DisplayOverrideSwitch
											key={metric}
											label={metricNames[metric]}
											value={settings.metricDisplayOverrides[metric] ?? "auto"}
											onChange={(value) => updateMetricDisplay(metric, value)}
										/>
									))}
								</div>
							</SettingsFieldset>
							<SettingsFieldset legend="Dimensions">
								<div className={styles.dimensionGroups}>
									{displayDimensionGroups.map((group) => (
										<section key={group.label} className={styles.dimensionGroup}>
											<h3>{group.label}</h3>
											<div>
												{group.dimensions.map((dimension) => (
													<DisplayOverrideSwitch
														key={dimension}
														label={dimensionNames[dimension]}
														value={settings.dimensionDisplayOverrides[dimension] ?? "auto"}
														onChange={(value) => updateDimensionDisplay(dimension, value)}
													/>
												))}
											</div>
										</section>
									))}
								</div>
							</SettingsFieldset>
						</div>
					</SettingsPanel>
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
	const [visitorGroupMode, setVisitorGroupMode] = useState<VisitorGroupMode | null>(null);
	const [trackSessions, setTrackSessions] = useState<boolean | null>(null);
	const [trackUtmParams, setTrackUtmParams] = useState<boolean | null>(null);
	const [trackGeo, setTrackGeo] = useState<GeoDetail | null>(null);
	const [dataRetention, setDataRetention] = useState<DataRetention>({ mode: "inherit" });
	const [error, setError] = useState<string>();

	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	useEffect(() => {
		if (!entity) return;
		const projects = entity.projects.map((project) => ({ value: project.id, label: project.displayName }));
		setDisplayName(entity.displayName);
		setSelectedProjects(projects);
		api["/api/dashboard/entity/{entity_id}/settings"]
			.get({ params: { entity_id: entity.id } })
			.json()
			.then((res) => {
				setSettings(res.settings);
				setVisitorGroupMode(res.settings.visitorGroupMode ?? null);
				setTrackSessions(res.settings.trackSessions ?? null);
				setTrackUtmParams(res.settings.trackUtmParams ?? null);
				setTrackGeo(res.settings.trackGeo ?? null);
				setDataRetention(res.settings.dataRetention);
			})
			.catch((err) => setError(err instanceof Error ? err.message : "Failed to load entity settings"));
	}, [entity]);

	const saveEntity = (nextDisplayName: string, nextProjects: Tag[]) => {
		if (!entity) return;
		api["/api/dashboard/entity/{entity_id}"]
			.put({
				params: { entity_id: entity.id },
				json: { displayName: nextDisplayName, projects: nextProjects.map((tag) => String(tag.value)) },
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

	const saveEntitySettings = (next: EntityCollectionSettings) => {
		if (!entity) return;
		setSettings(next);
		api["/api/dashboard/entity/{entity_id}/settings"]
			.put({
				params: { entity_id: entity.id },
				json: next,
			})
			.then(() => createToast("Entity collection updated", "success"))
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update entity collection settings");
				createToast("Failed to update entity collection", "error");
			});
	};

	const saveCollectionSettings = (next: Partial<EntityCollectionSettings>) => {
		if (!entity || !settings) return;
		saveEntitySettings({
			entityId: entity.id,
			visitorGroupMode,
			trackSessions,
			trackUtmParams,
			trackGeo,
			dataRetention,
			ingestDropRules: settings.ingestDropRules,
			...next,
		});
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
					<SettingsField label="Entity name" description="Used in the dashboard." name="displayName">
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
									value={visitorGroupMode}
									onChange={(value) => {
										setVisitorGroupMode(value);
										saveCollectionSettings({ visitorGroupMode: value });
									}}
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
									value={trackSessions == null ? "inherit" : String(trackSessions)}
									onChange={(event) => {
										const next = event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true";
										setTrackSessions(next);
										saveCollectionSettings({ trackSessions: next });
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
									value={trackUtmParams == null ? "inherit" : String(trackUtmParams)}
									onChange={(event) => {
										const next = event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true";
										setTrackUtmParams(next);
										saveCollectionSettings({ trackUtmParams: next });
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
									value={trackGeo}
									onChange={(value) => {
										setTrackGeo(value);
										saveCollectionSettings({ trackGeo: value });
									}}
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
									value={retentionValue(dataRetention)}
									onChange={(event) => {
										const next = event.currentTarget.value;
										if (!(retentionValues as readonly string[]).includes(next)) return;
										if (next === "inherit") {
											const dataRetention = { mode: "inherit" } as const;
											setDataRetention(dataRetention);
											saveCollectionSettings({ dataRetention });
										} else if (next === "keep_all") {
											const dataRetention = { mode: "all" } as const;
											setDataRetention(dataRetention);
											saveCollectionSettings({ dataRetention });
										} else {
											const dataRetention = { mode: "days", days: Number(next) } as const;
											setDataRetention(dataRetention);
											saveCollectionSettings({ dataRetention });
										}
									}}
								>
									<option value="inherit">Inherit global</option>
									<option value="keep_all">Keep all history</option>
									<option value="30">1 month</option>
									<option value="90">3 months</option>
									<option value="180">6 months</option>
									<option value="365">1 year</option>
									<option value="730">2 years</option>
								</select>
							</SettingsField>
						</SettingsPanel>
						<SettingsPanel value="filters">
							<FiltersEditor
								rules={settings.ingestDropRules}
								setRules={(ingestDropRules) => saveCollectionSettings({ ingestDropRules })}
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
