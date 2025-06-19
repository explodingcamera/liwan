import { User2Icon } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { api, useMe, useMutation } from "../../api";
import { createToast } from "../toast";
import styles from "./me.module.css";

export const MyAccount = () => {
	const newPasswordId = useId();
	const confirmPasswordId = useId();

	const formRef = useRef<HTMLFormElement>(null);
	const { role, username, isLoading } = useMe();
	const { mutate } = useMutation({
		mutationFn: api["/api/dashboard/user/{username}/password"].put,
		onSuccess: () => {
			createToast("Password updated successfully", "success");
			formRef.current?.reset();
		},
		onError: console.error,
	});

	const [loading, setLoading] = useState(true);
	useEffect(() => setLoading(isLoading), [isLoading]);
	if (loading || !username) return <div className={"loading-spinner"} />;

	const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
		e.preventDefault();
		const data = new FormData(e.currentTarget);
		const newPassword = data.get("newPassword") as string;
		const confirmNewPassword = data.get("confirmNewPassword") as string;
		if (newPassword !== confirmNewPassword) {
			createToast("Passwords do not match", "error");
			return;
		}
		mutate({ json: { password: newPassword }, params: { username } });
	};

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
				<code>
					<span className={styles.tag}>{"<script"}</span> type="module" data-entity="
					<span className={styles.entity}>YOUR_ENTITY_ID</span>" src="
					{window.location.origin}/script.js"
					<span className={styles.tag}>
						{">"}
						{"</script>"}
					</span>
				</code>
			</article>
			<article>
				<form className={styles.password} onSubmit={handleSubmit} ref={formRef}>
					<h2>Update Password</h2>
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
						<button type="submit" className="secondary">
							Update Password
						</button>
					</div>
				</form>
			</article>
		</div>
	);
};
