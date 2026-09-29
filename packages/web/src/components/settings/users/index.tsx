import styles from "../settings.module.css";

import { useEffect, useState } from "react";
import { SettingsIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath, basePath } from "@/config";
import { invalidateTeams, invalidateUsers, useTeams, useUsers } from "@/hooks/api";
import { getUsername } from "@/utils";
import { DeleteDialog } from "../dialogs";
import { SettingsForm, SettingsHeader, SettingsSwitch } from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export { CreateUser } from "./dialogs";

const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.slice(basePath.length).replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};

const SettingsLink = ({ href, label }: { href: string; label: string }) => {
	return (
		<a href={href} className={styles.settingsLink} aria-label={label} title={label}>
			<SettingsIcon size={18} />
		</a>
	);
};

export const UsersTable = () => {
	const { users, isLoading, authError } = useUsers();
	const rows = users.map((user) => ({ id: user.username, ...user })) ?? [];

	if (authError) {
		return "You don't have permission to view this page.";
	}

	const columns: Column<(typeof rows)[number]>[] = [
		{
			id: "username",
			header: "Username",
			render: (row) => <a href={appPath(`/settings/users/${row.username}`)}>{row.username}</a>,
			nowrap: true,
		},
		{
			id: "role",
			header: "Role",
			render: (row) => row.role,
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={appPath(`/settings/users/${row.username}`)} label={`Open ${row.username} settings`} />
			),
		},
	];

	return <Table columns={columns} rows={rows} isLoading={isLoading} />;
};

export const UserSettingsPage = ({ username: usernameProp }: { username: string }) => {
	const [username, setUsername] = useState<string>();

	useEffect(() => {
		setUsername(getSettingsPathId("/settings/users/") || usernameProp);
	}, [usernameProp]);

	if (!username) return <LoadingSpinner />;
	return <UserSettingsContent key={username} username={username} />;
};

const UserSettingsContent = ({ username }: { username: string }) => {
	const { users, isLoading, authError } = useUsers();
	const { teams, isLoading: teamsLoading, error: teamsError } = useTeams();
	const user = users.find((u) => u.username === username);
	const [isAdmin, setIsAdmin] = useState(false);
	const [selectedTeams, setSelectedTeams] = useState<Tag[]>([]);
	const [initialized, setInitialized] = useState(false);
	const teamTags = teams.map((team) => ({ value: team.id, label: team.displayName }));
	const { mutate: saveUser } = useMutation({
		scope: { id: `user:${username}` },
		mutationFn: api["/api/dashboard/user/{username}"].put,
		onSuccess: () => {
			invalidateUsers();
			invalidateTeams();
			createToast("User updated", "success");
		},
		onError: (error) => createToast(error.message, "error"),
	});

	const isSelf = getUsername() === username;
	const { mutate: revokeSessions, isPending: revoking } = useMutation({
		mutationFn: api["/api/dashboard/user/{username}/sessions"].delete,
		onSuccess: () => {
			createToast("Sessions revoked", "success");
			if (isSelf) window.location.href = appPath("/login");
		},
		onError: () => createToast("Failed to revoke sessions", "error"),
	});

	useEffect(() => {
		if (!user || isLoading || teamsLoading || initialized) return;
		setIsAdmin(user.role === "admin");
		setSelectedTeams(
			teams
				.filter((team) => team.users.includes(username))
				.map((team) => ({ value: team.id, label: team.displayName })),
		);
		setInitialized(true);
	}, [user, teams, username, isLoading, teamsLoading, initialized]);

	const updateUser = (nextIsAdmin: boolean, nextTeams: Tag[]) => {
		if (!user) return;
		saveUser({
			params: { username: user.username },
			json: { role: nextIsAdmin ? "admin" : "user", teams: nextTeams.map((tag) => tag.value) },
		});
	};

	if (authError) return <p>You don't have permission to view this page.</p>;
	if (teamsError) return <p>Could not load teams.</p>;
	if (isLoading || teamsLoading) return <LoadingSpinner />;
	if (!user) return <p>User not found.</p>;
	if (!initialized) return <LoadingSpinner />;

	return (
		<SettingsForm>
			<SettingsHeader title={user.username} backHref={appPath("/settings/users")} backLabel="Back to users" />
			<div className={`${styles.detailPanel} ${styles.userDetailPanel}`}>
				<SettingsSwitch
					label="Administrator access"
					description={
						<>
							Allow this user to manage projects, entities, teams, and users.
							{isSelf && " You cannot change your own role."}
						</>
					}
					checked={isAdmin}
					disabled={isSelf}
					onCheckedChange={(checked) => {
						setIsAdmin(checked);
						updateUser(checked, selectedTeams);
					}}
				/>
				<Tags
					labelText="Teams"
					labelDescription="Team membership grants access to the team's projects."
					selected={selectedTeams.map((tag) => ({
						...tag,
						label: teams.find((team) => team.id === tag.value)?.displayName ?? tag.label,
					}))}
					suggestions={teamTags}
					onAdd={(tag) => {
						const next = [...selectedTeams, tag];
						setSelectedTeams(next);
						updateUser(isAdmin, next);
					}}
					onDelete={(index) => {
						const next = selectedTeams.filter((_, i) => i !== index);
						setSelectedTeams(next);
						updateUser(isAdmin, next);
					}}
				/>
				<div className={styles.dangerZone}>
					<div>
						<strong>Revoke sessions</strong>
						<p>Sign this user out on all devices without changing their password.</p>
					</div>
					<button
						type="button"
						className="button-secondary"
						disabled={revoking}
						onClick={() => revokeSessions({ params: { username: user.username } })}
					>
						Revoke sessions
					</button>
				</div>
				<div className={styles.dangerZone}>
					<div>
						<strong>Delete user</strong>
						<p>The user will immediately lose access to the dashboard.</p>
					</div>
					{isSelf ? (
						<button
							type="button"
							className={`${styles.deleteButton} button-danger`}
							onClick={() => createToast("You cannot delete your own account", "error")}
						>
							Delete user
						</button>
					) : (
						<DeleteDialog
							id={user.username}
							displayName={user.username}
							type="user"
							onDeleted={() => {
								window.location.href = appPath("/settings/users");
							}}
							trigger={
								<button type="button" className={`${styles.deleteButton} button-danger`}>
									Delete user
								</button>
							}
						/>
					)}
				</div>
			</div>
		</SettingsForm>
	);
};
