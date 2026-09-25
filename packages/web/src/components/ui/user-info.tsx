import menuStyles from "./menu.module.css";
import styles from "./user-info.module.css";

import { Menu } from "@base-ui/react/menu";
import {
	ChevronDownIcon,
	HelpCircle,
	LogOutIcon,
	SettingsIcon,
	SquareArrowOutUpRightIcon,
	UserIcon,
} from "lucide-react";

import { api, queryClient } from "@/api";
import { getUsername } from "@/utils";

export const LoginButton = () => {
	const username = getUsername();
	const returnTo = `${window.location.pathname}${window.location.search}${window.location.hash}`;
	if (!username)
		return (
			<a className={styles.loginLink} href={`/login?${new URLSearchParams({ returnTo })}`}>
				Login
			</a>
		);

	return (
		<Menu.Root>
			<Menu.Trigger className={styles.trigger}>
				<UserIcon size="24" />
				{username}
				<ChevronDownIcon size="16" aria-hidden="true" />
			</Menu.Trigger>
			<Menu.Portal>
				<Menu.Positioner className={menuStyles.positioner} align="end" sideOffset={4}>
					<Menu.Popup className={menuStyles.popup}>
						<Menu.LinkItem className={menuStyles.item} href="/settings/me">
							<UserIcon size="16" /> My Account
						</Menu.LinkItem>
						<Menu.LinkItem className={menuStyles.item} href="/settings/projects">
							<SettingsIcon size="16" /> Admin
						</Menu.LinkItem>
						<Menu.LinkItem className={menuStyles.item} href="https://liwan.dev" target="_blank" rel="noreferrer">
							<HelpCircle size="16" /> Help
							<SquareArrowOutUpRightIcon size="16" className={styles.external} />
						</Menu.LinkItem>
						<Menu.Separator className={menuStyles.separator} />
						<Menu.Item
							className={menuStyles.item}
							onClick={() => {
								api["/api/dashboard/auth/logout"].post().then(() => {
									queryClient.clear();
									window.location.href = "/";
								});
							}}
						>
							<LogOutIcon size="16" /> Logout
						</Menu.Item>
					</Menu.Popup>
				</Menu.Positioner>
			</Menu.Portal>
		</Menu.Root>
	);
};
