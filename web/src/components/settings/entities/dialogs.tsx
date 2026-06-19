import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { useRef } from "react";
import { navigate } from "astro:transitions/client";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateEntities, useMe } from "@/hooks/api";
import { cls } from "@/utils";

export const CreateEntity = () => {
	const closeRef = useRef<HTMLButtonElement>(null);
	const { role } = useMe();
	const { mutate, error, reset } = useMutation({
		mutationFn: api["/api/dashboard/entity"].post,
		onSuccess: (_res, variables) => {
			createToast("Entity created", "success");
			invalidateEntities();
			navigate(`/settings/entities/${variables.json.id}`);
		},
		onError: console.error,
	});

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
		const { id, displayName } = Object.fromEntries(new FormData(event.currentTarget)) as {
			id: string;
			displayName: string;
		};
		mutate({ json: { id, displayName, projects: [] } });
	};

	return (
		<Dialog
			onOpenChange={() => reset()}
			title="Create a new entity"
			description="Entities are individual websites, apps, or services that send events. The entity ID is used in the tracking snippet."
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
					Entity ID <small>(cannot be changed later)</small>
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
					Entity name <small>(used in the dashboard)</small>
					<input required name="displayName" type="text" placeholder="My Website" autoComplete="off" />
				</label>
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create entity</button>
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
