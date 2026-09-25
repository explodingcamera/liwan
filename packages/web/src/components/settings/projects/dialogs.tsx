import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { navigate } from "astro:transitions/client";
import { PlusIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateProjects, useMe } from "@/hooks/api";

type ProjectVisibility = "private" | "unlisted" | "public";
const visibilityPublic = (visibility: ProjectVisibility) => visibility === "public" || visibility === "unlisted";

export const CreateProject = () => {
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
					<button type="button" className={styles.new} aria-label="Create project" title="Create project">
						<PlusIcon size={24} strokeWidth={2.25} />
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Project ID
					<small>Used in dashboard URLs and cannot be changed later.</small>
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
					Project name
					<small>Identifies this project in the dashboard.</small>
					<input required name="displayName" type="text" placeholder="My Project" autoComplete="off" />
				</label>
				<label>
					Visibility
					<small>Unlisted projects are public by direct link, but hidden from public project lists.</small>
					<select name="visibility" defaultValue="private">
						<option value="private">Private</option>
						<option value="unlisted">Unlisted</option>
						<option value="public">Public</option>
					</select>
				</label>

				<div className="action-row">
					<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
					<button type="submit" className="button-primary">
						Create project
					</button>
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
