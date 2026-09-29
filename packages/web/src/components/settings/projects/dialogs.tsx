import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { navigate } from "astro:transitions/client";
import { PlusIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { appPath } from "@/config";
import type { ProjectVisibility } from "@/constants";
import { invalidateProjects } from "@/hooks/api";
import { SettingsField } from "../form";

export const CreateProject = () => {
	const { mutate } = useMutation({
		mutationFn: api["/api/dashboard/project/{project_id}"].post,
		onSuccess: (_res, variables) => {
			createToast("Project created", "success");
			invalidateProjects();
			navigate(appPath(`/settings/projects/${variables.params.project_id}`));
		},
		onError: (error) => createToast(error.message, "error"),
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
				visibility,
				entities: [],
			},
		});
	};

	return (
		<Dialog
			title="Create a new project"
			description="Projects group one or more entities for reporting and access control."
			trigger={
				<button type="button" className={styles.new} aria-label="Create project" title="Create project">
					<PlusIcon size={24} strokeWidth={2.25} />
				</button>
			}
		>
			<form onSubmit={handleSubmit}>
				<SettingsField label="Project ID" description="Used in dashboard URLs and cannot be changed later.">
					<input
						required
						pattern="^[A-Za-z0-9_\-.]{1,40}$"
						name="id"
						type="text"
						placeholder="my-project"
						autoComplete="off"
					/>
				</SettingsField>
				<SettingsField label="Project name" description="Identifies this project in the dashboard.">
					<input required name="displayName" type="text" placeholder="My Project" autoComplete="off" />
				</SettingsField>
				<SettingsField
					label="Visibility"
					description="Unlisted projects are public by direct link, but hidden from public project lists."
				>
					<select name="visibility" defaultValue="private">
						<option value="private">Private</option>
						<option value="unlisted">Unlisted</option>
						<option value="internal">Internal</option>
						<option value="public">Public</option>
					</select>
				</SettingsField>

				<div className="action-row">
					<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
					<button type="submit" className="button-primary">
						Create project
					</button>
				</div>
			</form>
		</Dialog>
	);
};
