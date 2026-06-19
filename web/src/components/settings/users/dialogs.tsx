import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { useRef } from "react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateUsers, useMe } from "@/hooks/api";
import { cls } from "@/utils";

export const CreateUser = () => {
	const { role } = useMe();
	const closeRef = useRef<HTMLButtonElement>(null);

	const { mutate, error } = useMutation({
		mutationFn: api["/api/dashboard/user"].post,
		onSuccess: () => {
			closeRef?.current?.click();
			createToast("User created", "success");
			invalidateUsers();
		},
		onError: console.error,
	});

	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		event.stopPropagation();
		const { username, password, admin } = Object.fromEntries(new FormData(event.currentTarget)) as {
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
			description="Non-admin users can only view projects they belong to. They cannot create or edit projects, entities, or users."
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
					Username <small>(cannot be changed later)</small>
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
					<input name="admin" type="checkbox" role="switch" />
					Enable administrator access
					<br />
					<small>Administrators can edit and create projects, entities, and users.</small>
				</label>
				<br />
				<div className="grid">
					<Dialog.Close className="secondary outline" ref={closeRef}>
						Cancel
					</Dialog.Close>
					<button type="submit">Create user</button>
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
