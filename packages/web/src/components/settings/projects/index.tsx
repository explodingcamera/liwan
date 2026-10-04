import styles from "../settings.module.css";

import { Fragment, useEffect, useMemo, useState } from "react";
import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";

import { api } from "@/api";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath } from "@/config";
import type { Dimension, DisplayOverride, ProjectDisplaySettings, ProjectVisibility } from "@/constants";
import { dimensionNames, displayOverrides, metricNames, metrics } from "@/constants";
import { invalidateProjects, useEntities, useProjects } from "@/hooks/api";
import { DeleteDialog } from "../dialogs";
import {
	getSettingsPathId,
	SettingsField,
	SettingsForm,
	SettingsHeader,
	SettingsLink,
	SettingsPanel,
	SettingsTabs,
} from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export { CreateProject } from "./dialogs";

type ProjectTab = "general" | "display";
type DisplayKey = "metricDisplayOverrides" | "dimensionDisplayOverrides";

const visibilityLabels: Record<ProjectVisibility, string> = {
	private: "Private",
	public: "Public",
	unlisted: "Unlisted",
	internal: "Internal",
};

const displayLabels: Record<DisplayOverride, string> = {
	auto: "Auto",
	show: "Always",
	hide: "Hidden",
};
const displayDimensionGroups = [
	{
		label: "Campaigns",
		dimensions: ["referrer", "utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"],
	},
	{ label: "Device", dimensions: ["platform", "browser", "mobile", "screen_width", "orientation"] },
	{ label: "Pages", dimensions: ["url", "url_entry", "url_exit", "fqdn"] },
	{ label: "Other", dimensions: ["country", "city", "property"] },
] as const satisfies readonly {
	label: string;
	dimensions: readonly Dimension[];
}[];
const projectTabs = [
	{ value: "general", label: "General" },
	{ value: "display", label: "Display" },
] as const satisfies readonly { value: ProjectTab; label: string }[];

export const ProjectsTable = () => {
	const { projects, isLoading } = useProjects();

	const columns: Column<(typeof projects)[number]>[] = [
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={appPath(`/settings/projects/${row.id}`)}>{row.displayName}</a>,
			nowrap: true,
		},
		{
			id: "visibility",
			header: "Visibility",
			render: (row) => visibilityLabels[row.visibility],
		},
		{
			id: "entities",
			header: "Entities",
			render: (row) => (
				<>
					{row.entities.map((entity, i) => (
						<Fragment key={entity.id}>
							{i > 0 && ", "}
							<a href={appPath(`/settings/entities/${entity.id}`)}>{entity.displayName}</a>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={appPath(`/settings/projects/${row.id}`)} label={`Open ${row.displayName} settings`} />
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

	if (!resolvedProjectId) return <LoadingSpinner />;
	return <ProjectSettingsContent projectId={resolvedProjectId} />;
};

const ProjectSettingsContent = ({ projectId }: { projectId: string }) => {
	const { projects, isLoading } = useProjects();
	const { entities } = useEntities();
	const project = projects.find((project) => project.id === projectId);
	const [tab, setTab] = useState<ProjectTab>("general");
	const [form, setForm] = useState({
		displayName: "",
		visibility: "private" as ProjectVisibility,
		selectedEntities: [] as Tag[],
	});
	const { displayName, visibility, selectedEntities } = form;
	const [settings, setSettings] = useState<ProjectDisplaySettings>();

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
		setForm({
			displayName: project.displayName,
			visibility: project.visibility,
			selectedEntities: project.entities.map((entity) => ({
				value: entity.id,
				label: entity.displayName,
			})),
		});
		api["/api/dashboard/project/{project_id}/settings"]
			.get({ params: { project_id: project.id } })
			.json()
			.then(setSettings)
			.catch(() => createToast("Failed to load project settings", "error"));
	}, [project]);

	const saveProject = (nextDisplayName: string, nextVisibility: ProjectVisibility, nextEntities: Tag[]) => {
		if (!project) return;
		api["/api/dashboard/project/{project_id}"]
			.put({
				params: { project_id: project.id },
				json: {
					project: {
						displayName: nextDisplayName,
						visibility: nextVisibility,
					},
					entities: nextEntities.map((tag) => String(tag.value)),
				},
			})
			.then(() => {
				invalidateProjects();
				createToast("Project updated", "success");
			})
			.catch(() => createToast("Failed to update project", "error"));
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
			.catch(() => createToast("Failed to update project display", "error"));
	};

	const setDisplay = (key: DisplayKey, name: string, display: DisplayOverride) => {
		if (!project || !settings) return;
		const overrides = { ...settings[key] };
		if (display === "auto") delete overrides[name];
		else overrides[name] = display;
		saveProjectSettings({ ...settings, projectId: project.id, [key]: overrides });
	};

	const displayRow = (key: DisplayKey, name: string, label: string) => (
		<div className={styles.displayRow} key={name}>
			<span>{label}</span>
			<ToggleGroup
				aria-label={`${label} display`}
				className={styles.segmented}
				value={[settings?.[key][name] ?? "auto"]}
				onValueChange={(values) => {
					const next = values.at(-1);
					if ((displayOverrides as readonly string[]).includes(next ?? "")) {
						setDisplay(key, name, next as DisplayOverride);
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
	);

	if (isLoading) return <LoadingSpinner />;
	if (!project) return <p>Project not found.</p>;

	return (
		<SettingsForm>
			<SettingsHeader
				title={displayName || project.displayName}
				backHref={appPath("/settings/projects")}
				backLabel="Back to projects"
			/>
			<SettingsTabs value={tab} onValueChange={setTab} tabs={projectTabs}>
				<SettingsPanel value="general" className={styles.detailPanel}>
					<SettingsField
						label="Project name *"
						description="Identifies this project in the dashboard."
						name="displayName"
					>
						<input
							required
							name="displayName"
							type="text"
							value={displayName}
							onChange={(event) => setForm({ ...form, displayName: event.currentTarget.value })}
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
								setForm({ ...form, visibility: next });
								saveProject(displayName, next, selectedEntities);
							}}
						>
							<option value="private">Private</option>
							<option value="unlisted">Unlisted</option>
							<option value="internal">Internal</option>
							<option value="public">Public</option>
						</select>
					</SettingsField>
					<Tags
						labelText="Associated entities"
						labelDescription="Entities that send analytics data to this project."
						selected={selectedEntities}
						suggestions={entityTags}
						onAdd={(tag) => {
							const next = [...selectedEntities, tag];
							setForm({ ...form, selectedEntities: next });
							saveProject(displayName, visibility, next);
						}}
						onDelete={(i) => {
							const next = selectedEntities.filter((_, index) => index !== i);
							setForm({ ...form, selectedEntities: next });
							saveProject(displayName, visibility, next);
						}}
					/>
					<div className={styles.dangerZone}>
						<div>
							<strong>Delete project</strong>
							<p>The project will be deleted. Its entities and event data will remain.</p>
						</div>
						<DeleteDialog
							id={project.id}
							displayName={project.displayName}
							type="project"
							onDeleted={() => {
								window.location.href = appPath("/settings/projects");
							}}
							trigger={
								<button type="button" className="button-danger">
									Delete project
								</button>
							}
						/>
					</div>
				</SettingsPanel>
				{settings && (
					<SettingsPanel value="display">
						<p className={styles.displayHelp}>Auto follows data availability. Always may show partial results.</p>
						<div className={styles.displaySections}>
							<fieldset>
								<legend>Metrics</legend>
								<div className={styles.displayGrid}>
									{metrics.map((metric) => displayRow("metricDisplayOverrides", metric, metricNames[metric]))}
								</div>
							</fieldset>
							<fieldset>
								<legend>Dimensions</legend>
								<div className={styles.dimensionGroups}>
									{displayDimensionGroups.map((group) => (
										<section className={styles.dimensionGroup} key={group.label}>
											<h3>{group.label}</h3>
											{group.dimensions.map((dimension) =>
												displayRow("dimensionDisplayOverrides", dimension, dimensionNames[dimension]),
											)}
											{group.label === "Other" &&
												displayRow("metricDisplayOverrides", "custom_events", "Custom Events")}
										</section>
									))}
								</div>
							</fieldset>
						</div>
					</SettingsPanel>
				)}
			</SettingsTabs>
		</SettingsForm>
	);
};
