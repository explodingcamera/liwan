import type { ReactElement, SubmitEvent } from "react";
import { useRef } from "react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateEntities, invalidateProjects, invalidateTeams, invalidateUsers } from "@/hooks/api";

const toTitleCase = (str: string) => str[0].toUpperCase() + str.slice(1);

export const DeleteDialog = ({
	id,
	displayName,
	type,
	trigger,
	onDeleted,
}: {
	id: string;
	displayName: string;
	type: "project" | "entity" | "user" | "team";
	trigger: ReactElement;
	onDeleted?: () => void;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);

	const endpoints = {
		project: (id: string) =>
			api["/api/dashboard/project/{project_id}"].delete({
				params: { project_id: id },
			}),
		entity: (id: string) =>
			api["/api/dashboard/entity/{entity_id}"].delete({
				params: { entity_id: id },
			}),
		user: (id: string) =>
			api["/api/dashboard/user/{username}"].delete({
				params: { username: id },
			}),
		team: (id: string) => api["/api/dashboard/team/{team_id}"].delete({ params: { team_id: id } }),
	} as const;

	const { mutate } = useMutation({
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
				case "team":
					invalidateTeams();
					break;
			}
			createToast(`${toTitleCase(type)} deleted`, "success");
			onDeleted?.();
		},
		onError: (error) => createToast(error.message, "error"),
	});

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
		mutate();
	};

	return (
		<Dialog
			title={`Delete ${toTitleCase(type)}: ${displayName}`}
			description={`Are you sure you want to delete this ${type}?\n ${
				type === "entity" ? "This will not delete the data associated with it." : "This action cannot be undone."
			}`}
			trigger={trigger}
		>
			<form onSubmit={handleSubmit}>
				<div className="action-row">
					<Dialog.Close className="button-secondary" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit" className="button-danger">
						Delete {type}
					</button>
				</div>
			</form>
		</Dialog>
	);
};
