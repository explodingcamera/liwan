import styles from "./me.module.css";
import { api, useMe, useMutation } from "../../api";
import { User2Icon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createToast } from "../toast";

export const MyAccount = () => {
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
				<form className={styles.password} onSubmit={handleSubmit} ref={formRef}>
					<h2>Update Password</h2>
					<label>
						New Password
						<input
							minLength={8}
							required
							type="password"
							id="newPassword"
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
							id="confirmNewPassword"
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
