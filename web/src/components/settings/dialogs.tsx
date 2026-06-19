import styles from "./dialogs.module.css";

import type { ReactElement, SubmitEvent } from "react";
import { useRef } from "react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateEntities, invalidateProjects, invalidateUsers, useMe } from "@/hooks/api";

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
	type: "project" | "entity" | "user";
	trigger: ReactElement;
	onDeleted?: () => void;
}) => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();

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
			createToast(`${toTitleCase(type)} deleted`, "success");
			onDeleted?.();
		},
		onError: console.error,
	});

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
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
