import styles from "../settings.module.css";

import { useEffect, useMemo, useState } from "react";
import { SettingsIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath, basePath } from "@/config";
import { invalidateUsers, useProjects, useUsers } from "@/hooks/api";
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
	return <UserSettingsContent username={username} />;
};

const UserSettingsContent = ({ username }: { username: string }) => {
	const { users, isLoading, authError } = useUsers();
	const { projects } = useProjects();
	const user = users.find((u) => u.username === username);
	const [form, setForm] = useState({ selectedProjects: [] as Tag[], isAdmin: false });
	const { selectedProjects, isAdmin } = form;

	const projectTags = useMemo(() => projects.map((p) => ({ value: p.id, label: p.displayName })), [projects]);

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
		if (!user) return;
		setForm({
			isAdmin: user.role === "admin",
			selectedProjects: user.projects.map((projectId) => {
				const p = projects.find((p) => p.id === projectId);
				return { value: projectId, label: p ? p.displayName : projectId };
			}),
		});
	}, [user, projects]);

	const saveUser = (nextProjects: Tag[], nextIsAdmin: boolean) => {
		if (!user) return;
		setForm({ selectedProjects: nextProjects, isAdmin: nextIsAdmin });
		api["/api/dashboard/user/{username}"]
			.put({
				params: { username: user.username },
				json: {
					role: nextIsAdmin ? "admin" : "user",
					projects: nextProjects.map((tag) => tag.value as string),
				},
			})
			.then(() => {
				invalidateUsers();
				createToast("User updated", "success");
			})
			.catch(() => createToast("Failed to update user", "error"));
	};

	if (authError) return <p>You don't have permission to view this page.</p>;
	if (isLoading) return <LoadingSpinner />;
	if (!user) return <p>User not found.</p>;

	return (
		<SettingsForm>
			<SettingsHeader title={user.username} backHref={appPath("/settings/users")} backLabel="Back to users" />
			<div className={`${styles.detailPanel} ${styles.userDetailPanel}`}>
				<div className={styles.projectAccess}>
					<Tags
						labelText="Project access"
						labelDescription="Choose which projects this user can view."
						selected={selectedProjects}
						suggestions={projectTags}
						onAdd={(tag) => saveUser([...selectedProjects, tag], isAdmin)}
						onDelete={(i) =>
							saveUser(
								selectedProjects.filter((_, index) => i !== index),
								isAdmin,
							)
						}
					/>
				</div>
				<SettingsSwitch
					label="Administrator access"
					description={
						<>
							Allow this user to manage projects, entities, and users.
							{isSelf && " You cannot change your own role."}
						</>
					}
					checked={isAdmin}
					disabled={isSelf}
					onCheckedChange={(checked) => saveUser(selectedProjects, checked)}
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
