import { useMemo, useRef, useState } from "react";
import { Dialog } from "../dialog";
import {
	api,
	invalidateEntities,
	invalidateProjects,
	invalidateUsers,
	queryClient,
	useMe,
	useMutation,
	useProjects,
	type ProjectResponse,
} from "../../api";

import styles from "./dialogs.module.css";
import { Tags, type Tag } from "../tags";

export const DeleteDialog = ({
	id,
	displayName,
	type,
	trigger,
}: { id: string; displayName: string; type: "project" | "entity" | "user"; trigger: JSX.Element }) => {
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
			title={`Delete ${type}: ${displayName}`}
			description={`Are you sure you want to delete this ${type}?\n ${
				type === "entity" ? "This will not delete the data associated with it." : "This action cannot be undone."
			}`}
			trigger={role === "admin" && trigger}
		>
			<form onSubmit={handleSubmit}>
				<div className="grid">
					<Dialog.Close asChild>
						<button className="secondary outline" type="button" ref={closeRef}>
							Cancel
						</button>
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

export const EditProject = ({ project, trigger }: { project: ProjectResponse; trigger: JSX.Element }) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const [proj, setProj] = useState(project);
	const { role } = useMe();

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/project/{project_id}"].put,
		onSuccess: () => {
			closeRef?.current?.click();
			queryClient.invalidateQueries({ queryKey: ["projects"] });
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { displayName, isPublic } = Object.fromEntries(new FormData(form)) as {
			displayName: string;
			isPublic: string;
		};
		mutate({ params: { project_id: project.id }, json: { displayName, public: isPublic === "on" } });
	};
};

export const CreateProject = () => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/project/{project_id}"].post,
		onSuccess: () => {
			closeRef?.current?.click();
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
					<button type="button" className="contrast">
						New
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Project ID <small>(This cannot be changed later)</small>
					<input required pattern="^[A-Za-z0-9_\-]{1,15}$" name="id" type="text" placeholder="my-project" />
				</label>
				<label>
					Project Name <small>(Used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Project" />
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
					<Dialog.Close asChild>
						<button className="secondary outline" type="button" ref={closeRef}>
							Cancel
						</button>
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

export const CreateEntity = () => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();
	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/entity"].post,
		onSuccess: () => {
			closeRef?.current?.click();
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
					<button type="button" className="contrast">
						New
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Entity ID <small>(This cannot be changed later)</small>
					<input required pattern="^[A-Za-z0-9_\-]{1,15}$" name="id" type="text" placeholder="my-website" />
				</label>
				<label>
					Entity Name <small>(Used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Website" />
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
					<Dialog.Close asChild>
						<button className="secondary outline" type="button" ref={closeRef}>
							Cancel
						</button>
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

export const CreateUser = () => {
	const { role } = useMe();
	const closeRef = useRef<HTMLButtonElement>(null);

	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/user"].post,
		onSuccess: () => {
			closeRef?.current?.click();
			invalidateUsers();
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { username, password } = Object.fromEntries(new FormData(form)) as { username: string; password: string };
	};

	return (
		<Dialog
			title="Create a new user"
			description="Non-admin users can only view data of projects they are members of, and cannot create or edit projects, entities, or users."
			trigger={
				role === "admin" && (
					<button type="button" className="contrast">
						New
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Username <small>(This cannot be changed later)</small>
					<input required pattern="^[A-Za-z0-9_\-]{1,15}$" name="username" type="text" placeholder="MyUsername" />
				</label>
				<label>
					Password
					<input required name="password" type="password" />
				</label>

				<label>
					{/* biome-ignore lint/a11y/useAriaPropsForRole: this is an uncontrolled component */}
					<input name="publish" type="checkbox" role="switch" />
					Enable Administrator Access
					<br />
					<small>Administators can edit and create projects, entities, and users.</small>
				</label>
				<br />
				<div className="grid">
					<Dialog.Close asChild>
						<button className="secondary outline" type="button" ref={closeRef}>
							Cancel
						</button>
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
