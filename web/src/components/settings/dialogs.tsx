import { useRef } from "react";
import { Dialog } from "../dialog";

export const CreateProject = () => {
	const closeRef = useRef<HTMLButtonElement>(null);

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		e.stopPropagation();
		const form = e.target as HTMLFormElement;
		const { id, displayname } = Object.fromEntries(new FormData(form)) as { id: string; displayname: string };
	};

	return (
		<Dialog
			title="Create a new project"
			trigger={
				<button type="button" className="contrast">
					New
				</button>
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
					<input required name="displayname" type="text" placeholder="My Project" />
				</label>
				<div className="grid">
					<Dialog.Close asChild>
						<button className="secondary outline" type="button" ref={closeRef}>
							Cancel
						</button>
					</Dialog.Close>
					<button type="submit">Create Project</button>
				</div>
			</form>
		</Dialog>
	);
};

export const CreateEntity = () => {
	const closeRef = useRef<HTMLButtonElement>(null);

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
				<button type="button" className="contrast">
					New
				</button>
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
			</form>
		</Dialog>
	);
};

export const CreateUser = () => {
	const closeRef = useRef<HTMLButtonElement>(null);

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
				<button type="button" className="contrast">
					New
				</button>
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
			</form>
		</Dialog>
	);
};
