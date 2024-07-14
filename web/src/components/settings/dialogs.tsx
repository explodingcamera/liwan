import { useRef, useState } from "react";
import { Dialog } from "../dialog";
import { api, queryClient, useMe, useMutation, useQuery, type ProjectResponse } from "../../api";
import styles from "./dialogs.module.css";

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
			queryClient.invalidateQueries({ queryKey: ["projects"] });
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
			json: { displayName, public: isPublic === "on" },
		});
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title="Create a new project"
			trigger={
				role === "admin" && (
					<button type="button" className="contrast">
						New
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<p>
					Project's are a collection of entities that you want to track and are used to group data from different
					sources together.
				</p>

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
			queryClient.invalidateQueries({ queryKey: ["entities"] });
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { id, displayname } = Object.fromEntries(new FormData(form)) as { id: string; displayname: string };
	};

	return (
		<Dialog
			title="Create a new entity"
			trigger={
				role === "admin" && (
					<button type="button" className="contrast">
						New
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<p>
					Entities are individual clients or services that you want to track, like distinct websites or mobile apps.
				</p>

				<label>
					Entity ID <small>(This cannot be changed later)</small>
					<input required pattern="^[A-Za-z0-9_\-]{1,15}$" name="id" type="text" placeholder="my-website" />
				</label>
				<label>
					Project Name <small>(Used in the dashboard)</small>
					<input required name="displayname" type="text" placeholder="My Website" />
				</label>
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
			queryClient.invalidateQueries({ queryKey: ["users"] });
		},
		onError: console.error,
	});

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { email, password } = Object.fromEntries(new FormData(form)) as { email: string; password: string };
	};

	return (
		<Dialog
			title="Create a new user"
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
