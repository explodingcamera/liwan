import { type ReactElement, useEffect, useMemo, useRef, useState } from "react";
import type { OASModel } from "fets";

import { Dialog } from "../dialog";
import { type Tag, Tags } from "../tags";
import styles from "./dialogs.module.css";

import {
	type EntityResponse,
	type ProjectResponse,
	type UserResponse,
	api,
	invalidateEntities,
	invalidateProjects,
	invalidateUsers,
	queryClient,
	dimensions,
	metrics,
	type Metric,
	type DashboardSpec,
	useEntities,
	useMe,
	useMutation,
	useProjects,
} from "../../api";
import { FiltersEditor, GeoSelect, VisitorModeSelect } from "./collection";
import { cls } from "../../utils";
import { createToast } from "../toast";

const toTitleCase = (str: string) => str[0].toUpperCase() + str.slice(1);

type EntityCollectionSettings = OASModel<DashboardSpec, "EntityCollectionSettings">;
type CollectionSettings = OASModel<DashboardSpec, "CollectionSettings">;
type VisitorGroupMode = CollectionSettings["visitorGroupMode"];
type GeoDetail = CollectionSettings["trackGeo"];
type DataRetention = EntityCollectionSettings["dataRetention"];
type ProjectDisplaySettings = OASModel<DashboardSpec, "ProjectDisplaySettings">;
type DisplayOverride = ProjectDisplaySettings["metricDisplayOverrides"][string];

const displayOverrides = ["auto", "show", "hide"] as const satisfies readonly DisplayOverride[];
const entityRetentionOptions = [
	{ value: "inherit", label: "Inherit global" },
	{ value: "keep_all", label: "Keep all history" },
	{ value: "30", label: "1 month" },
	{ value: "90", label: "3 months" },
	{ value: "180", label: "6 months" },
	{ value: "365", label: "1 year" },
	{ value: "730", label: "2 years" },
] as const;
const entityRetentionValues = entityRetentionOptions.map((option) => option.value);
const entityRetentionValue = (retention: DataRetention) => {
	if (retention.mode === "inherit") return "inherit";
	if (retention.mode === "all") return "keep_all";
	const value = String(retention.days);
	return isOneOf(entityRetentionValues, value) ? value : "365";
};
const title = (value: string) => value.replaceAll("_", " ").replace(/\b\w/g, (char) => char.toUpperCase());
const isOneOf = <T extends string>(values: readonly T[], value: string): value is T =>
	values.some((item) => item === value);

export const DeleteDialog = ({
	id,
	displayName,
	type,
	trigger,
	onDeleted,
}: {
	id: string;
	displayName: string;
	type: "project" | "entity" | "user";
	trigger: ReactElement;
	onDeleted?: () => void;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();

	const endpoints = {
		project: (id: string) => api["/api/dashboard/project/{project_id}"].delete({ params: { project_id: id } }),
		entity: (id: string) => api["/api/dashboard/entity/{entity_id}"].delete({ params: { entity_id: id } }),
		user: (id: string) => api["/api/dashboard/user/{username}"].delete({ params: { username: id } }),
	} as const;

	const { mutate, error, reset } = useMutation({
		mutationFn: () => endpoints[type](id),
		onSuccess: () => {
			closeRef?.current?.click();
			switch (type) {
				case "project":
					invalidateProjects();
					break;
				case "entity":
					invalidateEntities();
					break;
				case "user":
					invalidateUsers();
					break;
			}
			createToast(`${toTitleCase(type)} deleted successfully`, "success");
			onDeleted?.();
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		mutate({ params: { [`${type}_id`]: id } });
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title={`Delete ${toTitleCase(type)}: ${displayName}`}
			description={`Are you sure you want to delete this ${type}?\n ${
				type === "entity" ? "This will not delete the data associated with it." : "This action cannot be undone."
			}`}
			trigger={role === "admin" && trigger}
		>
			<form onSubmit={handleSubmit}>
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit" className={styles.danger}>
						Delete {type}
					</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while deleting this "}
						{type}
						{":"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

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
		if (!isOneOf(displayOverrides, value)) return;
		setProjectSettings((settings) => {
			if (!settings) return settings;
			const metricDisplayOverrides = { ...settings.metricDisplayOverrides };
			if (value === "auto") delete metricDisplayOverrides[metric];
			else metricDisplayOverrides[metric] = value;
			return { ...settings, metricDisplayOverrides };
		});
	};

	const updateDimensionDisplay = (dimension: string, value: string) => {
		if (!isOneOf(displayOverrides, value)) return;
		setProjectSettings((settings) => {
			if (!settings) return settings;
			const dimensionDisplayOverrides = { ...settings.dimensionDisplayOverrides };
			if (value === "auto") delete dimensionDisplayOverrides[dimension];
			else dimensionDisplayOverrides[dimension] = value;
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

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { displayName, isPublic } = Object.fromEntries(new FormData(form)) as {
			displayName: string;
			isPublic: string;
		};

		try {
			await api["/api/dashboard/project/{project_id}"].put({
				params: { project_id: project.id },
				json: {
					project: { displayName, public: isPublic === "on" },
					entities: selectedEntities.map((tag) => tag.value as string),
				},
			});

			if (projectSettings) {
				await api["/api/dashboard/project/{project_id}/settings"].put({
					params: { project_id: project.id },
					json: {
						projectId: project.id,
						metricDisplayOverrides: projectSettings.metricDisplayOverrides,
						dimensionDisplayOverrides: projectSettings.dimensionDisplayOverrides,
					},
				});
			}

			closeRef.current?.click();
			queryClient.invalidateQueries({ queryKey: ["projects"] });
			createToast("Project updated successfully", "success");
		} catch (err) {
			setError(err instanceof Error ? err : new Error("Failed to update project"));
		}
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
			description="Edit the project's name or change its visibility."
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
						Project Name <small>(Used in the dashboard)</small>
						<input required name="displayName" type="text" defaultValue={project.displayName} autoComplete="off" />
					</label>
					<Tags
						labelText="Associated Entities"
						selected={selectedEntities}
						suggestions={entityTags}
						onAdd={(tag) => setSelectedEntities((rest) => [...rest, tag])}
						onDelete={(i) => setSelectedEntities(selectedEntities.filter((_, index) => index !== i))}
						noOptionsText="No matching entities"
					/>
					<label>
						{/* biome-ignore lint/a11y/useAriaPropsForRole: this is an uncontrolled component */}
						<input type="checkbox" role="switch" name="isPublic" defaultChecked={project.public} />
						Make Public <br />
						<small>Public projects can be viewed by anyone, even if they are not logged in.</small>
					</label>
				</section>
				{projectSettings && (
					<section className={styles.tabPanel} hidden={tab !== "display"}>
						<p>
							Auto hides metrics or dimensions when project data is incomplete. Use Show anyway only when partial
							results are acceptable.
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
					<button type="submit">Save Changes</button>
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
		onSuccess: () => {
			closeRef?.current?.click();
			createToast("Project created successfully", "success");
			invalidateProjects();
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { id, displayName, isPublic } = Object.fromEntries(new FormData(form)) as {
			id: string;
			displayName: string;
			isPublic: string;
		};

		mutate({
			params: { project_id: id },
			json: { displayName, public: isPublic === "on", entities: [] },
		});
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title="Create a new project"
			description="Project's are a collection of entities that you want to track and are used to group data from different
					sources together."
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
					Project ID <small>(This cannot be changed later)</small>
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
					Project Name <small>(Used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Project" autoComplete="off" />
				</label>
				<label>
					{/* biome-ignore lint/a11y/useAriaPropsForRole: this is an uncontrolled component */}
					<input type="checkbox" role="switch" name="isPublic" />
					Make Public
					<br />
					<small>Public projects can be viewed by anyone, even if they are not logged in.</small>
				</label>
				<br />

				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create Project</button>
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

export const EditEntity = ({ entity, trigger }: { entity: EntityResponse; trigger: ReactElement }) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();
	const [error, setError] = useState<Error>();
	const [tab, setTab] = useState<"general" | "collection" | "filters">("general");
	const [entitySettings, setEntitySettings] = useState<EntityCollectionSettings>();
	const [visitorGroupMode, setVisitorGroupMode] = useState<VisitorGroupMode | null>(null);
	const [trackSessions, setTrackSessions] = useState<boolean | null>(null);
	const [trackUtmParams, setTrackUtmParams] = useState<boolean | null>(null);
	const [trackGeo, setTrackGeo] = useState<GeoDetail | null>(null);
	const [dataRetention, setDataRetention] = useState<DataRetention>({ mode: "inherit" });

	const { projects } = useProjects();
	const projectTags = useMemo(() => projects.map((p) => ({ value: p.id, label: p.displayName })), [projects]);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);

	useEffect(() => {
		setSelectedProjects(
			entity.projects.map((project) => ({
				value: project.id,
				label: project.displayName,
			})),
		);
	}, [entity.projects]);

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { displayName } = Object.fromEntries(new FormData(form)) as { displayName: string };

		const submitProjects = selectedProjects.map((tag) => tag.value as string);
		const updateProjects =
			entity.projects
				.map((p) => p.id)
				.sort()
				.join() === submitProjects.sort().join();

		try {
			await api["/api/dashboard/entity/{entity_id}"].put({
				params: { entity_id: entity.id },
				json: { displayName, projects: updateProjects ? undefined : submitProjects },
			});

			if (entitySettings) {
				await api["/api/dashboard/entity/{entity_id}/settings"].put({
					params: { entity_id: entity.id },
					json: {
						entityId: entity.id,
						visitorGroupMode,
						trackSessions,
						trackUtmParams,
						trackGeo,
						dataRetention,
						ingestDropRules: entitySettings.ingestDropRules,
					},
				});
			}

			closeRef.current?.click();
			createToast("Entity updated successfully", "success");
			invalidateEntities();
		} catch (err) {
			setError(err instanceof Error ? err : new Error("Failed to update entity"));
		}
	};

	return (
		<Dialog
			onOpenChange={(open) => {
				setError(undefined);
				if (open) {
					setTab("general");
					api["/api/dashboard/entity/{entity_id}/settings"]
						.get({ params: { entity_id: entity.id } })
						.json()
						.then((res) => {
							setEntitySettings(res.settings);
							setVisitorGroupMode(res.settings.visitorGroupMode ?? null);
							setTrackSessions(res.settings.trackSessions ?? null);
							setTrackUtmParams(res.settings.trackUtmParams ?? null);
							setTrackGeo(res.settings.trackGeo ?? null);
							setDataRetention(res.settings.dataRetention);
						})
						.catch((err) => setError(err instanceof Error ? err : new Error("Failed to load entity settings")));
				}
			}}
			title={entity.displayName}
			description="Edit the entity and how its data is collected."
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
						className={cls(styles.tab, tab === "collection" && styles.activeTab)}
						onClick={() => setTab("collection")}
					>
						Collection
					</button>
					<button
						type="button"
						className={cls(styles.tab, tab === "filters" && styles.activeTab)}
						onClick={() => setTab("filters")}
					>
						Filters
					</button>
				</div>

				<section className={styles.tabPanel} hidden={tab !== "general"}>
					<label>
						Entity Name <small>(Used in the dashboard)</small>
						<input required name="displayName" type="text" defaultValue={entity.displayName} autoComplete="off" />
					</label>
					<Tags
						labelText="Associated Projects"
						selected={selectedProjects}
						suggestions={projectTags}
						onAdd={(tag) => setSelectedProjects((rest) => [...rest, tag])}
						onDelete={(i) => setSelectedProjects(selectedProjects.filter((_, index) => index !== i))}
						noOptionsText="No matching projects"
					/>
				</section>

				{entitySettings && (
					<>
						<section className={styles.tabPanel} hidden={tab !== "collection"}>
							<label htmlFor="entityVisitorGroupMode">
								Visitor grouping
								<VisitorModeSelect
									id="entityVisitorGroupMode"
									value={visitorGroupMode}
									onChange={setVisitorGroupMode}
									allowInherit
								/>
								<small>Controls how repeat visits are grouped for this entity.</small>
							</label>
							<label>
								Session metrics
								<select
									name="trackSessions"
									value={trackSessions == null ? "inherit" : String(trackSessions)}
									onChange={(event) =>
										setTrackSessions(
											event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true",
										)
									}
								>
									<option value="inherit">Inherit global</option>
									<option value="true">Track</option>
									<option value="false">Do not track</option>
								</select>
								<small>Required for bounce rate, time on site, entry URL, and exit URL.</small>
							</label>
							<label>
								UTM parameters
								<select
									name="trackUtmParams"
									value={trackUtmParams == null ? "inherit" : String(trackUtmParams)}
									onChange={(event) =>
										setTrackUtmParams(
											event.currentTarget.value === "inherit" ? null : event.currentTarget.value === "true",
										)
									}
								>
									<option value="inherit">Inherit global</option>
									<option value="true">Track</option>
									<option value="false">Do not track</option>
								</select>
								<small>Stores campaign fields like source, medium, campaign, term, and content.</small>
							</label>
							<label htmlFor="entityTrackGeo">
								Geolocation detail
								<GeoSelect id="entityTrackGeo" value={trackGeo} onChange={setTrackGeo} allowInherit />
								<small>Choose how much location data is stored for this entity.</small>
							</label>
							<fieldset>
								<div className="grid">
									<label>
										History retention
										<select
											name="historyRetention"
											value={entityRetentionValue(dataRetention)}
											onChange={(event) => {
												const next = event.currentTarget.value;
												if (!isOneOf(entityRetentionValues, next)) return;
												if (next === "inherit") {
													setDataRetention({ mode: "inherit" });
												} else if (next === "keep_all") {
													setDataRetention({ mode: "all" });
												} else {
													setDataRetention({ mode: "days", days: Number(next) });
												}
											}}
										>
											{entityRetentionOptions.map((option) => (
												<option key={option.value} value={option.value}>
													{option.label}
												</option>
											))}
										</select>
									</label>
								</div>
								<small>Automatically prune older event data after the selected period.</small>
							</fieldset>
						</section>

						<section className={styles.tabPanel} hidden={tab !== "filters"}>
							<FiltersEditor
								rules={entitySettings.ingestDropRules}
								setRules={(ingestDropRules) => setEntitySettings({ ...entitySettings, ingestDropRules })}
								scope="entity"
							/>
						</section>
					</>
				)}

				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Save Changes</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while editing the entity:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

export const CreateEntity = () => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();
	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/entity"].post,
		onSuccess: () => {
			closeRef?.current?.click();
			createToast("Entity created successfully", "success");
			invalidateEntities();
		},
		onError: console.error,
	});

	const { projects } = useProjects();
	const projectTags = useMemo(() => projects.map((p) => ({ value: p.id, label: p.displayName })), [projects]);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { id, displayName } = Object.fromEntries(new FormData(form)) as { id: string; displayName: string };
		mutate({ json: { id, displayName, projects: selectedProjects.map((tag) => tag.value as string) } });
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title="Create a new entity"
			description="Entities are individual clients or services that you want to track, like distinct websites or mobile apps. The entity id is used in the tracking snippet to identify the source of the data."
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
					Entity ID <small>(This cannot be changed later)</small>
					<input
						required
						pattern="^[A-Za-z0-9_\-.]{1,40}$"
						name="id"
						type="text"
						placeholder="my-website"
						autoComplete="off"
					/>
				</label>
				<label>
					Entity Name <small>(Used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Website" autoComplete="off" />
				</label>
				<Tags
					labelText="Add to Projects"
					selected={selectedProjects}
					suggestions={projectTags}
					onAdd={(tag) => setSelectedProjects((rest) => [...rest, tag])}
					onDelete={(i) => setSelectedProjects(selectedProjects.filter((_, index) => index !== i))}
					noOptionsText="No matching projects"
				/>
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create Entity</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while creating the entity:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

export const EditPassword = ({ user, trigger }: { user: UserResponse; trigger: ReactElement }) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const confirmPasswordRef = useRef<HTMLInputElement>(null);
	const { role } = useMe();

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/user/{username}/password"].put,
		onSuccess: () => {
			createToast("Password updated successfully", "success");
			closeRef?.current?.click();
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { password, confirm } = Object.fromEntries(new FormData(form)) as { password: string; confirm: string };

		if (password !== confirm) {
			confirmPasswordRef.current?.setCustomValidity("Passwords do not match");
			confirmPasswordRef.current?.reportValidity();
			return;
		}

		confirmPasswordRef.current?.setCustomValidity("");
		mutate({ params: { username: user.username }, json: { password } });
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title={`Change Password: ${user.username}`}
			description="Enter a new password for the user."
			hideDescription
			trigger={role === "admin" && trigger}
		>
			<form onSubmit={handleSubmit}>
				<label>
					New Password
					<input minLength={8} required name="password" type="password" autoComplete="new-password" />
				</label>
				<label>
					Confirm New Password
					<input required name="confirm" type="password" autoComplete="new-password" ref={confirmPasswordRef} />
				</label>
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Change Password</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while changing the user's password:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

export const EditUser = ({ user, trigger }: { user: UserResponse; trigger: ReactElement }) => {
	const closeRef = useRef<HTMLButtonElement>(null);

	const { projects } = useProjects();
	const projectTags = useMemo(() => projects.map((p) => ({ value: p.id, label: p.displayName })), [projects]);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/user/{username}"].put,
		onSuccess: () => {
			closeRef?.current?.click();
			createToast("User updated successfully", "success");
			invalidateUsers();
		},
		onError: console.error,
	});

	// biome-ignore lint/correctness/useExhaustiveDependencies: don't want to re-run this effect when projects change
	useEffect(() => {
		setSelectedProjects(
			user.projects.map((projectId) => {
				const p = projects.find((p) => p.id === projectId);
				return {
					value: projectId,
					label: p ? p.displayName : projectId,
				};
			}),
		);
	}, [user.projects]);

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();

		const form = e.target as HTMLFormElement;
		const { admin } = Object.fromEntries(new FormData(form)) as { admin: string };
		const role = admin === "on" ? "admin" : "user";

		mutate({
			params: { username: user.username },
			json: { role, projects: selectedProjects.map((tag) => tag.value as string) },
		});
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title={`Edit User: ${user.username}`}
			description="Edit the user's role."
			hideDescription
			trigger={trigger}
		>
			<form onSubmit={handleSubmit}>
				<Tags
					labelText="Projects"
					selected={selectedProjects}
					suggestions={projectTags}
					onAdd={(tag) => setSelectedProjects((rest) => [...rest, tag])}
					onDelete={(i) => setSelectedProjects(selectedProjects.filter((_, index) => index !== i))}
					noOptionsText="No matching projects"
				/>
				<label>
					{/* biome-ignore lint/a11y/useAriaPropsForRole: this is an uncontrolled component */}
					<input name="admin" type="checkbox" role="switch" defaultChecked={user.role === "admin"} />
					Enable Administrator Access
					<br />
					<small>Administrators can edit and create projects, entities, and users.</small>
				</label>
				<br />

				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Save Changes</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while editing the user:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};

export const CreateUser = () => {
	const { role } = useMe();
	const closeRef = useRef<HTMLButtonElement>(null);

	const { mutate, error } = useMutation({
		mutationFn: api["/api/dashboard/user"].post,
		onSuccess: () => {
			closeRef?.current?.click();
			createToast("User created successfully", "success");
			invalidateUsers();
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { username, password, admin } = Object.fromEntries(new FormData(form)) as {
			username: string;
			password: string;
			admin: string;
		};
		const role = admin === "on" ? "admin" : "user";
		mutate({ json: { username, password, role } });
	};

	return (
		<Dialog
			title="Create a new user"
			description="Non-admin users can only view data of projects they are members of, and cannot create or edit projects, entities, or users."
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
					Username <small>(This cannot be changed later)</small>
					<input
						required
						pattern="^[A-Za-z0-9_\-]{1,20}$"
						name="username"
						type="text"
						placeholder="MyUsername"
						autoComplete="username"
					/>
				</label>
				<label>
					Password
					<input required name="password" type="password" autoComplete="new-password" minLength={8} />
				</label>
				<label>
					{/* biome-ignore lint/a11y/useAriaPropsForRole: this is an uncontrolled component */}
					<input name="publish" type="checkbox" role="switch" />
					Enable Administrator Access
					<br />
					<small>Administrators can edit and create projects, entities, and users.</small>
				</label>
				<br />
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create User</button>
				</div>
				{error && (
					<article role="alert" className={styles.error}>
						{"An error occurred while creating the user:"}
						<br />
						{error?.message ?? "Unknown error"}
					</article>
				)}
			</form>
		</Dialog>
	);
};
