import styles from "./me.module.css";

import type { SubmitEvent } from "react";
import { useEffect, useId, useRef } from "react";
import { User2Icon } from "lucide-react";

import { api, useMutation } from "@/api";
import { LoadingSpinner } from "@/components/ui/loading";
import { createToast } from "@/components/ui/toast";
import { useMe } from "@/hooks/api";
import { getUsername } from "@/utils";

export const MyAccount = () => {
	const currentPasswordId = useId();
	const newPasswordId = useId();
	const confirmPasswordId = useId();
	const formRef = useRef<HTMLFormElement>(null);

	const { role, username: queriedUsername, authError } = useMe();
	const username = queriedUsername ?? getUsername();
	useEffect(() => {
		if (authError) createToast("You don't have permission to view this page", "error");
	}, [authError]);

	const { mutate, isPending } = useMutation({
		mutationFn: api["/api/dashboard/auth/me/password"].put,
		onSuccess: () => {
			createToast("Password updated", "success");
			formRef.current?.reset();
		},
		onError: (error) => createToast(error.message, "error"),
	});

	const updatePassword = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!username) return;

		const data = new FormData(event.currentTarget);
		const newPassword = data.get("newPassword") as string;
		const currentPassword = data.get("currentPassword") as string;
		const confirmNewPassword = data.get("confirmNewPassword") as string;
		if (newPassword !== confirmNewPassword) {
			createToast("Passwords do not match", "error");
			return;
		}

		mutate({ json: { password: newPassword, currentPassword } });
	};

	if (authError) return null;

	if (!username) return <LoadingSpinner />;

	return (
		<div className={styles.container}>
			<article>
				<nav>
					<h1>My Account</h1>
				</nav>

				<div className={styles.header}>
					<User2Icon size={48} />
					<div>
						<h2>{username}</h2>
						<p data-loading={!role}>Role: {role === "admin" ? "Administrator" : "User"}</p>
					</div>
				</div>
			</article>
			<article>
				<form className={styles.password} onSubmit={updatePassword} ref={formRef}>
					<h2>Update password</h2>
					<p>Changing your password signs you out on other devices.</p>
					<label>
						Current password *
						<input
							required
							type="password"
							id={currentPasswordId}
							name="currentPassword"
							autoComplete="current-password"
						/>
					</label>
					<label>
						New password *
						<input
							minLength={8}
							required
							type="password"
							id={newPasswordId}
							name="newPassword"
							autoComplete="new-password"
						/>
					</label>

					<label>
						Confirm new password *
						<input
							minLength={8}
							required
							type="password"
							id={confirmPasswordId}
							name="confirmNewPassword"
							autoComplete="new-password"
						/>
					</label>

					<div className={styles.passwordActions}>
						<button type="submit" className="button-primary" disabled={isPending}>
							Update password
						</button>
					</div>
				</form>
			</article>
		</div>
	);
};
