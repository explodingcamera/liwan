import { User2Icon } from "lucide-react";
import { useId, useRef, useState } from "react";
import { api, useMe } from "../../api";
import { createToast } from "../toast";
import styles from "./me.module.css";
import { Snippet } from "./snippet";

export const MyAccount = () => {
	const newPasswordId = useId();
	const confirmPasswordId = useId();

	const formRef = useRef<HTMLFormElement>(null);
	const { role, username, isLoading, authError } = useMe();
	const [passwordError, setPasswordError] = useState<string | null>(null);
	const [passwordUpdating, setPasswordUpdating] = useState(false);

	const updatePassword = async (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!username) {
			setPasswordError("You must be logged in to update your password");
			return;
		}

		const data = new FormData(event.currentTarget);
		const newPassword = data.get("newPassword");
		const confirmNewPassword = data.get("confirmNewPassword");
		if (typeof newPassword !== "string" || typeof confirmNewPassword !== "string") return;
		if (newPassword !== confirmNewPassword) {
			setPasswordError("Passwords do not match");
			return;
		}

		setPasswordUpdating(true);
		setPasswordError(null);
		try {
			await api["/api/dashboard/user/{username}/password"].put({
				json: { password: newPassword },
				params: { username },
			});
			createToast("Password updated successfully", "success");
			formRef.current?.reset();
		} catch (err) {
			setPasswordError(err instanceof Error ? err.message : "Failed to update password");
		} finally {
			setPasswordUpdating(false);
		}
	};

	if (authError) {
		return "You don't have permission to view this page.";
	}

	if (isLoading || !username) return <div className={"loading-spinner"} />;

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
						<p>Role: {role === "admin" ? "Administrator" : "User"}</p>
					</div>
				</div>
			</article>
			<article>
				<h2>Snippet code</h2>
				<p>
					You can copy the tracking snippet for a specific entity <a href="/settings/entities">here</a>, use the{" "}
					<a href="https://npmjs.com/package/liwan-tracker">liwan-tracker</a> npm package, or use the following code:
				</p>
				<Snippet entityId="YOUR_ENTITY_ID" />
			</article>
			<article>
				<form className={styles.password} onSubmit={updatePassword} ref={formRef}>
					<h2>Update Password</h2>
					{passwordError && <article role="alert">{passwordError}</article>}
					<label>
						New Password
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
						Confirm New Password
						<input
							minLength={8}
							required
							type="password"
							id={confirmPasswordId}
							name="confirmNewPassword"
							autoComplete="new-password"
						/>
					</label>

					<div>
						<button type="submit" className="secondary" disabled={passwordUpdating}>
							{passwordUpdating ? "Updating..." : "Update Password"}
						</button>
					</div>
				</form>
			</article>
		</div>
	);
};
