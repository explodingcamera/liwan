import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { navigate } from "astro:transitions/client";
import { PlusIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateEntities, useMe } from "@/hooks/api";

export const CreateEntity = () => {
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
			description="Entities represent websites, apps, or services that send analytics events."
			trigger={
				role === "admin" && (
					<button type="button" className={styles.new} aria-label="Create entity" title="Create entity">
						<PlusIcon size={24} strokeWidth={2.25} />
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Entity ID
					<small>Used in the tracking snippet and cannot be changed.</small>
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
					Entity name
					<input required name="displayName" type="text" placeholder="My Website" autoComplete="off" />
				</label>
				<div className="grid">
					<Dialog.Close className="secondary outline">Cancel</Dialog.Close>
					<button type="submit" className="contrast">
						Create entity
					</button>
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
