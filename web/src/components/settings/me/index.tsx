import styles from "./me.module.css";

import type { SubmitEvent } from "react";
import { useEffect, useId, useRef, useState } from "react";
import { User2Icon } from "lucide-react";

import { FONT_SIZES, type FontSize, getStoredFontSize, setAppFontSize } from "@/components/ui/font-size-switcher";
import { api, useMutation } from "@/api";
import { Snippet } from "@/components/ui/snippet";
import { createToast } from "@/components/ui/toast";
import { useMe } from "@/hooks/api";
import { type TimeFormat, useTimeFormat } from "@/hooks/persist";

export const MyAccount = () => {
	const [fontSize, setFontSize] = useState<FontSize>("normal");
	const { timeFormat, setTimeFormat } = useTimeFormat();
	const newPasswordId = useId();
	const confirmPasswordId = useId();

	useEffect(() => {
		setFontSize(getStoredFontSize());
		const handleCustomChange = (e: Event) => {
			const customEvent = e as CustomEvent<FontSize>;
			if (customEvent.detail) setFontSize(customEvent.detail);
		};
		window.addEventListener("liwan:font-size", handleCustomChange);
		return () => window.removeEventListener("liwan:font-size", handleCustomChange);
	}, []);

	const formRef = useRef<HTMLFormElement>(null);
	const { role, username, isLoading, authError } = useMe();

	const { mutate, error } = useMutation({
		mutationFn: api["/api/dashboard/user/{username}/password"].put,
		onSuccess: () => {
			createToast("Password updated", "success");
			formRef.current?.reset();
		},
		onError: console.error,
	});

	const updatePassword = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!username) return;

		const data = new FormData(event.currentTarget);
		const newPassword = data.get("newPassword") as string;
		const confirmNewPassword = data.get("confirmNewPassword") as string;
		if (newPassword !== confirmNewPassword) {
			createToast("Passwords do not match", "error");
			return;
		}

		mutate({ json: { password: newPassword }, params: { username } });
	};

	if (authError) {
		return "You don't have permission to view this page.";
	}

	if (isLoading || !username) return <div className={"loading-spinner"} />;

	return (
		<div className={styles.container}>
			<article>
				<nav>
					<h1>My account</h1>
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
				<h2>Tracking snippet</h2>
				<p>
					Copy the tracking snippet for a specific entity from <a href="/settings/entities">entity settings</a>, use the{" "}
					<a href="https://npmjs.com/package/liwan-tracker">liwan-tracker</a> npm package, or start with this example:
				</p>
				<Snippet entityId="YOUR_ENTITY_ID" />
			</article>
			<article>
				<h2>Appearance</h2>
				<p>Customize the display font size and density across all dashboards.</p>
				<div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginTop: "0.75rem" }}>
					{FONT_SIZES.map((item) => (
						<button
							key={item.id}
							type="button"
							className={fontSize === item.id ? "contrast" : "outline secondary"}
							style={{ minWidth: "6.5rem", fontSize: "0.82rem", margin: 0 }}
							onClick={() => {
								setFontSize(item.id);
								setAppFontSize(item.id);
							}}
						>
							{item.label} ({item.scale})
						</button>
					))}
				</div>
			</article>
			<article>
				<h2>Time format</h2>
				<p>Choose between 12-hour (AM/PM) and 24-hour clock display for session activity timestamps.</p>
				<div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginTop: "0.75rem" }}>
					<button
						type="button"
						className={timeFormat === "12h" ? "contrast" : "outline secondary"}
						style={{ minWidth: "9.5rem", fontSize: "0.82rem", margin: 0 }}
						onClick={() => {
							setTimeFormat("12h");
							createToast("Time format set to 12-hour", "success");
						}}
					>
						12-hour (e.g. 2:30:15 PM)
					</button>
					<button
						type="button"
						className={timeFormat === "24h" ? "contrast" : "outline secondary"}
						style={{ minWidth: "9.5rem", fontSize: "0.82rem", margin: 0 }}
						onClick={() => {
							setTimeFormat("24h");
							createToast("Time format set to 24-hour", "success");
						}}
					>
						24-hour (e.g. 14:30:15)
					</button>
				</div>
			</article>
			<article>
				<form className={styles.password} onSubmit={updatePassword} ref={formRef}>
					<h2>Update password</h2>
					{error && <article role="alert">{error.message}</article>}
					<label>
						New password
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
						Confirm new password
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
						<button type="submit" className="contrast">
							Update password
						</button>
					</div>
				</form>
			</article>
		</div>
	);
};
