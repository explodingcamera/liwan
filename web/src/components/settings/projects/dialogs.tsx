import styles from "../dialogs.module.css";

import type { ReactElement, SubmitEvent } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { navigate } from "astro:transitions/client";

import { api, queryClient, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import type { DisplayOverride, Metric, ProjectDisplaySettings, ProjectResponse } from "@/constants";
import { dimensions, displayOverrides, metrics } from "@/constants";
import { invalidateProjects, useEntities, useMe } from "@/hooks/api";
import { cls } from "@/utils";
import type { Tag } from "../tags";
import { Tags } from "../tags";

const title = (value: string) => value.replaceAll("_", " ").replace(/\b\w/g, (char) => char.toUpperCase());
type ProjectVisibility = "private" | "unlisted" | "public";
const projectVisibility = (project: ProjectResponse): ProjectVisibility => {
	if (!project.public) return "private";
	return project.unlisted ? "unlisted" : "public";
};
const visibilityPublic = (visibility: ProjectVisibility) => visibility === "public" || visibility === "unlisted";

export const EditProject = ({ project, trigger }: { project: ProjectResponse; trigger: ReactElement }) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();
	const [tab, setTab] = useState<"general" | "display">("general");
	const [projectSettings, setProjectSettings] = useState<ProjectDisplaySettings>();
	const [error, setError] = useState<Error>();

	const { entities } = useEntities();
	const entityTags = useMemo(() => entities.map((p) => ({ value: p.id, label: p.displayName })), [entities]);
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);

	const updateMetricDisplay = (metric: Metric, value: string) => {
		if (!(displayOverrides as readonly string[]).includes(value)) return;
		setProjectSettings((settings) => {
			if (!settings) return settings;
			const metricDisplayOverrides = { ...settings.metricDisplayOverrides };
			const display = value as DisplayOverride;
			if (display === "auto") delete metricDisplayOverrides[metric];
			else metricDisplayOverrides[metric] = display;
			return { ...settings, metricDisplayOverrides };
		});
	};

	const updateDimensionDisplay = (dimension: string, value: string) => {
		if (!(displayOverrides as readonly string[]).includes(value)) return;
		setProjectSettings((settings) => {
			if (!settings) return settings;
			const dimensionDisplayOverrides = {
				...settings.dimensionDisplayOverrides,
			};
			const display = value as DisplayOverride;
			if (display === "auto") delete dimensionDisplayOverrides[dimension];
			else dimensionDisplayOverrides[dimension] = display;
			return { ...settings, dimensionDisplayOverrides };
		});
	};

	useEffect(() => {
		setSelectedEntities(
			project.entities.map((entity) => ({
				value: entity.id,
				label: entity.displayName,
			})),
		);
	}, [project.entities]);

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
		const { displayName, visibility } = Object.fromEntries(new FormData(event.currentTarget)) as {
			displayName: string;
			visibility: ProjectVisibility;
		};

		api["/api/dashboard/project/{project_id}"]
			.put({
				params: { project_id: project.id },
				json: {
					project: {
						displayName,
						public: visibilityPublic(visibility),
						unlisted: visibility === "unlisted",
					},
					entities: selectedEntities.map((tag) => tag.value as string),
				},
			})
			.then(() => {
				if (!projectSettings) return;
				return api["/api/dashboard/project/{project_id}/settings"].put({
					params: { project_id: project.id },
					json: {
						projectId: project.id,
						metricDisplayOverrides: projectSettings.metricDisplayOverrides,
						dimensionDisplayOverrides: projectSettings.dimensionDisplayOverrides,
					},
				});
			})
			.then(() => {
				closeRef.current?.click();
				queryClient.invalidateQueries({ queryKey: ["projects"] });
				createToast("Project updated", "success");
			})
			.catch((err) => setError(err instanceof Error ? err : new Error("Failed to update project")));
	};

	return (
		<Dialog
			onOpenChange={(open) => {
				setError(undefined);
				if (open) {
					setTab("general");
					api["/api/dashboard/project/{project_id}/settings"]
						.get({ params: { project_id: project.id } })
						.json()
						.then(setProjectSettings)
						.catch((err) => setError(err instanceof Error ? err : new Error("Failed to load project settings")));
				}
			}}
			title={project.displayName}
			description="Edit the project name, visibility, and display settings."
			hideDescription
			trigger={role === "admin" && trigger}
			autoOverflow
			className={styles.editDialog}
		>
			<form onSubmit={handleSubmit}>
				<div className={styles.tabs}>
					<button
						type="button"
						className={cls(styles.tab, tab === "general" && styles.activeTab)}
						onClick={() => setTab("general")}
					>
						General
					</button>
					<button
						type="button"
						className={cls(styles.tab, tab === "display" && styles.activeTab)}
						onClick={() => setTab("display")}
					>
						Display
					</button>
				</div>
				<section className={styles.tabPanel} hidden={tab !== "general"}>
					<label>
						Project name <small>(used in the dashboard)</small>
						<input required name="displayName" type="text" defaultValue={project.displayName} autoComplete="off" />
					</label>
					<Tags
						labelText="Associated entities"
						labelDescription="Controls which entities send data to this project."
						selected={selectedEntities}
						suggestions={entityTags}
						onAdd={(tag) => setSelectedEntities((rest) => [...rest, tag])}
						onDelete={(i) => setSelectedEntities(selectedEntities.filter((_, index) => index !== i))}
						noOptionsText="No matching entities"
					/>
					<label>
						Visibility
						<select name="visibility" defaultValue={projectVisibility(project)}>
							<option value="private">Private</option>
							<option value="unlisted">Unlisted</option>
							<option value="public">Public</option>
						</select>
						<small>Unlisted projects are public by direct link, but hidden from public project lists.</small>
					</label>
				</section>
				{projectSettings && (
					<section className={styles.tabPanel} hidden={tab !== "display"}>
						<p>
							Auto hides metrics or dimensions when project data is incomplete. Use Show anyway only if partial results
							are acceptable.
						</p>
						<h3>Metrics</h3>
						{metrics.map((metric) => (
							<label key={metric}>
								{title(metric)}
								<select
									name={`metric:${metric}`}
									value={projectSettings.metricDisplayOverrides[metric] ?? "auto"}
									onChange={(event) => updateMetricDisplay(metric, event.currentTarget.value)}
								>
									<option value="auto">Auto</option>
									<option value="show">Show anyway</option>
									<option value="hide">Hide</option>
								</select>
							</label>
						))}
						<h3>Dimensions</h3>
						{dimensions.map((dimension) => (
							<label key={dimension}>
								{title(dimension)}
								<select
									name={`dimension:${dimension}`}
									value={projectSettings.dimensionDisplayOverrides[dimension] ?? "auto"}
									onChange={(event) => updateDimensionDisplay(dimension, event.currentTarget.value)}
								>
									<option value="auto">Auto</option>
									<option value="show">Show anyway</option>
									<option value="hide">Hide</option>
								</select>
							</label>
						))}
					</section>
				)}
				<br />
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Save changes</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while editing the project:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

export const CreateProject = () => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/project/{project_id}"].post,
		onSuccess: (_res, variables) => {
			createToast("Project created", "success");
			invalidateProjects();
			navigate(`/settings/projects/${variables.params.project_id}`);
		},
		onError: console.error,
	});

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
		const { id, displayName, visibility } = Object.fromEntries(new FormData(event.currentTarget)) as {
			id: string;
			displayName: string;
			visibility: ProjectVisibility;
		};

		mutate({
			params: { project_id: id },
			json: {
				displayName,
				public: visibilityPublic(visibility),
				unlisted: visibility === "unlisted",
				entities: [],
			},
		});
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title="Create a new project"
			description="Projects group one or more entities for reporting and access control."
			trigger={
				role === "admin" && (
					<button type="button" className={cls("contrast", styles.new)}>
						Create
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Project ID <small>(cannot be changed later)</small>
					<input
						required
						pattern="^[A-Za-z0-9_\-.]{1,40}$"
						name="id"
						type="text"
						placeholder="my-project"
						autoComplete="off"
					/>
				</label>
				<label>
					Project name <small>(used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Project" autoComplete="off" />
				</label>
				<label>
					Visibility
					<select name="visibility" defaultValue="private">
						<option value="private">Private</option>
						<option value="unlisted">Unlisted</option>
						<option value="public">Public</option>
					</select>
					<small>Unlisted projects are public by direct link, but hidden from public project lists.</small>
				</label>
				<br />

				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create project</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while creating the project:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};
