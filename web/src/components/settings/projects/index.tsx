import styles from "../settings.module.css";

import { Fragment, useEffect, useMemo, useState } from "react";
import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";
import { SettingsIcon } from "lucide-react";

import { api } from "@/api";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import type { Dimension, DisplayOverride, ProjectDisplaySettings, ProjectResponse } from "@/constants";
import { dimensionNames, displayOverrides, metricNames, metrics } from "@/constants";
import { invalidateProjects, useEntities, useMe, useProjects } from "@/hooks/api";
import { DeleteDialog } from "../dialogs";
import { SettingsField, SettingsForm, SettingsHeader, SettingsPanel, SettingsTabs } from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export { CreateProject, EditProject } from "./dialogs";

type ProjectTab = "general" | "display";
type ProjectVisibility = "private" | "unlisted" | "public";

const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};
const projectVisibility = (project: ProjectResponse): ProjectVisibility => {
	if (!project.public) return "private";
	return project.unlisted ? "unlisted" : "public";
};
const visibilityPublic = (visibility: ProjectVisibility) => visibility === "public" || visibility === "unlisted";

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
] as const satisfies readonly {
	label: string;
	dimensions: readonly Dimension[];
}[];
const projectTabs = [
	{ value: "general", label: "General" },
	{ value: "display", label: "Display" },
] as const satisfies readonly { value: ProjectTab; label: string }[];

const SettingsLink = ({ href, label }: { href: string; label: string }) => {
	const { role } = useMe();
	if (role === "user") return null;

	return (
		<a href={href} className={styles.settingsLink} aria-label={label} title={label}>
			<SettingsIcon size={18} />
		</a>
	);
};

export const ProjectsTable = () => {
	const { projects, isLoading } = useProjects();

	const columns: Column<(typeof projects)[number]>[] = [
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={`/settings/projects/${row.id}`}>{row.displayName}</a>,
			nowrap: true,
		},
		{
			id: "public",
			header: "Visibility",
			render: (row) => <>{row.public ? (row.unlisted ? "Unlisted" : "Public") : "Private"}</>,
		},
		{
			id: "entities",
			header: "Entities",
			render: (row) => (
				<>
					{row.entities.map((entity, i) => (
						<Fragment key={entity.id}>
							{i > 0 && ", "}
							<a href={`/settings/entities/${entity.id}`}>{entity.displayName}</a>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={`/settings/projects/${row.id}`} label={`Open ${row.displayName} settings`} />
			),
		},
	];

	return <Table columns={columns} rows={projects} isLoading={isLoading} />;
};

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
	const [visibility, setVisibility] = useState<ProjectVisibility>("private");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [settings, setSettings] = useState<ProjectDisplaySettings>();
	const [error, setError] = useState<string>();

	const entityTags = useMemo(
		() =>
			entities.map((entity) => ({
				value: entity.id,
				label: entity.displayName,
			})),
		[entities],
	);

	useEffect(() => {
		if (!project) return;
		setDisplayName(project.displayName);
		setVisibility(projectVisibility(project));
		setSelectedEntities(
			project.entities.map((entity) => ({
				value: entity.id,
				label: entity.displayName,
			})),
		);
		api["/api/dashboard/project/{project_id}/settings"]
			.get({ params: { project_id: project.id } })
			.json()
			.then(setSettings)
			.catch((err) => setError(err instanceof Error ? err.message : "Failed to load project settings"));
	}, [project]);

	const saveProject = (nextDisplayName: string, nextVisibility: ProjectVisibility, nextEntities: Tag[]) => {
		if (!project) return;
		api["/api/dashboard/project/{project_id}"]
			.put({
				params: { project_id: project.id },
				json: {
					project: {
						displayName: nextDisplayName,
						public: visibilityPublic(nextVisibility),
						unlisted: nextVisibility === "unlisted",
					},
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
			.then(() => createToast("Project display updated", "success"))
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update project display settings");
				createToast("Failed to update project display", "error");
			});
	};

	const setMetricDisplay = (metric: string, display: DisplayOverride) => {
		if (!project || !settings) return;
		const metricDisplayOverrides = { ...settings.metricDisplayOverrides };
		if (display === "auto") delete metricDisplayOverrides[metric];
		else metricDisplayOverrides[metric] = display;
		saveProjectSettings({
			...settings,
			projectId: project.id,
			metricDisplayOverrides,
		});
	};

	const setDimensionDisplay = (dimension: string, display: DisplayOverride) => {
		if (!project || !settings) return;
		const dimensionDisplayOverrides = { ...settings.dimensionDisplayOverrides };
		if (display === "auto") delete dimensionDisplayOverrides[dimension];
		else dimensionDisplayOverrides[dimension] = display;
		saveProjectSettings({
			...settings,
			projectId: project.id,
			dimensionDisplayOverrides,
		});
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
						description="Unlisted projects are public by direct link, but hidden from public project lists."
						name="visibility"
					>
						<select
							name="visibility"
							value={visibility}
							onChange={(event) => {
								const next = event.currentTarget.value as ProjectVisibility;
								setVisibility(next);
								saveProject(displayName, next, selectedEntities);
							}}
						>
							<option value="private">Private</option>
							<option value="unlisted">Unlisted</option>
							<option value="public">Public</option>
						</select>
					</SettingsField>
					<Tags
						labelText="Associated entities"
						labelDescription="Controls which entities send data to this project."
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
							onDeleted={() => {
								window.location.href = "/settings/projects";
							}}
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
							Auto hides metrics or dimensions when project data is incomplete. Use Always only if partial results are
							acceptable.
						</p>
						<div className={styles.displaySections}>
							<fieldset>
								<legend>Metrics</legend>
								<div className={styles.displayGrid}>
									{metrics.map((metric) => (
										<div className={styles.displayRow} key={metric}>
											<span>{metricNames[metric]}</span>
											<ToggleGroup
												aria-label={`${metricNames[metric]} display`}
												className={styles.segmented}
												value={[settings.metricDisplayOverrides[metric] ?? "auto"]}
												onValueChange={(values) => {
													const next = values.at(-1);
													if ((displayOverrides as readonly string[]).includes(next ?? "")) {
														setMetricDisplay(metric, next as DisplayOverride);
													}
												}}
											>
												{displayOverrides.map((display) => (
													<Toggle key={display} value={display}>
														{displayLabels[display]}
													</Toggle>
												))}
											</ToggleGroup>
										</div>
									))}
								</div>
							</fieldset>
							<fieldset>
								<legend>Dimensions</legend>
								<div className={styles.dimensionGroups}>
									{displayDimensionGroups.map((group) => (
										<section className={styles.dimensionGroup} key={group.label}>
											<h3>{group.label}</h3>
											{group.dimensions.map((dimension) => (
												<div className={styles.displayRow} key={dimension}>
													<span>{dimensionNames[dimension]}</span>
													<ToggleGroup
														aria-label={`${dimensionNames[dimension]} display`}
														className={styles.segmented}
														value={[settings.dimensionDisplayOverrides[dimension] ?? "auto"]}
														onValueChange={(values) => {
															const next = values.at(-1);
															if ((displayOverrides as readonly string[]).includes(next ?? "")) {
																setDimensionDisplay(dimension, next as DisplayOverride);
															}
														}}
													>
														{displayOverrides.map((display) => (
															<Toggle key={display} value={display}>
																{displayLabels[display]}
															</Toggle>
														))}
													</ToggleGroup>
												</div>
											))}
										</section>
									))}
								</div>
							</fieldset>
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
