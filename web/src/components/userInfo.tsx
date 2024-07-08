import { getUsername } from "../api/utils";
import { LogOutIcon, SettingsIcon, UserIcon } from "lucide-react";
import styles from "./userInfo.module.css";
import { api } from "../api";

export const LoginButton = () => {
	const username = getUsername();
	if (!username)
		return (
			<li>
				<a href="/login">
					<button type="button" className="outline secondary">
						Login
					</button>
				</a>
				&nbsp;&nbsp;
			</li>
		);

	return (
		<details className={`dropdown ${styles.user}`}>
			<summary role="button" className="outline secondary">
				<UserIcon size="24" />
				{username}
			</summary>
			<ul>
				<li>
					<a href="/settings">
						<SettingsIcon size="16" />
						Settings
					</a>
				</li>
				<li>
					{/* biome-ignore lint/a11y/useValidAnchor: */}
					<a
						onClick={() => {
							api["/api/dashboard/auth/logout"].post().then(() => {
								window.location.href = "/";
							});
						}}
						href="#"
					>
						<LogOutIcon size="16" />
						Logout
					</a>
				</li>
			</ul>
		</details>
	);
};
