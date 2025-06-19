import styles from "./userInfo.module.css";

import { HelpCircle, LogOutIcon, SettingsIcon, SquareArrowOutUpRightIcon, UserIcon } from "lucide-react";
import { api } from "../api";
import { cls, getUsername } from "../utils";

export const LoginButton = () => {
	const username = getUsername();
	if (!username)
		return (
			<>
				<a href="/login">
					<button type="button" className="outline secondary">
						Login
					</button>
				</a>
				&nbsp;&nbsp;
			</>
		);

	return (
		<details className={cls("dropdown", "right", styles.user)}>
			<summary role="button" className="outline secondary">
				<UserIcon size="24" />
				{username}
			</summary>
			<ul>
				<li>
					<a href="/settings/me">
						<UserIcon size="16" />
						My Account
					</a>
				</li>
				<li>
					<a href="/settings/projects">
						<SettingsIcon size="16" />
						Admin
					</a>
				</li>
				<li>
					<a href="https://liwan.dev" target="_blank" rel="noreferrer">
						<HelpCircle size="16" />
						Help
						<SquareArrowOutUpRightIcon size="16" className={styles.external} />
					</a>
				</li>
				<li>
					<hr />
				</li>
				<li>
					{/* biome-ignore lint/a11y/useValidAnchor: no*/}
					<a
						href="#"
						onClick={() => {
							api["/api/dashboard/auth/logout"].post().then(() => {
								window.location.href = "/";
							});
						}}
					>
						<LogOutIcon size="16" />
						Logout
					</a>
				</li>
			</ul>
		</details>
	);
};
