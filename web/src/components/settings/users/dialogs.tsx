import styles from "../dialogs.module.css";

import type { SubmitEvent } from "react";
import { useRef } from "react";
import { PlusIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { createToast } from "@/components/ui/toast";
import { invalidateUsers, useMe } from "@/hooks/api";

export const CreateUser = () => {
	const { role } = useMe();
	const closeRef = useRef<HTMLButtonElement>(null);

	const { mutate, error, reset } = useMutation({
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
			onOpenChange={() => reset()}
			title="Create a new user"
			description="Users can access assigned projects unless administrator access is enabled."
			trigger={
				role === "admin" && (
					<button type="button" className={styles.new} aria-label="Create user" title="Create user">
						<PlusIcon size={24} strokeWidth={2.25} />
					</button>
				)
			}
		>
			<form onSubmit={handleSubmit}>
				<label>
					Username
					<small>Cannot be changed later.</small>
					<input
						required
						pattern="^[A-Za-z0-9_\-]{2,20}$"
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
					<button type="submit" className="contrast">
						Create user
					</button>
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
