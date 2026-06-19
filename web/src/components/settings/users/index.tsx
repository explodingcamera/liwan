import styles from "../settings.module.css";

import { useEffect, useMemo, useState } from "react";
import { SettingsIcon } from "lucide-react";

import { api } from "@/api";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { invalidateUsers, useMe, useProjects, useUsers } from "@/hooks/api";
import { getUsername } from "@/utils";
import { DeleteDialog } from "../dialogs";
import { SettingsField, SettingsForm, SettingsHeader, SettingsSwitch } from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export { CreateUser } from "./dialogs";

const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};

const SettingsLink = ({ href, label }: { href: string; label: string }) => {
	const { role } = useMe();
	if (role === "user") return null;
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
			render: (row) => <a href={`/settings/users/${row.username}`}>{row.username}</a>,
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
				<SettingsLink href={`/settings/users/${row.username}`} label={`Open ${row.username} settings`} />
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

	if (!username) return <div className="loading-spinner" />;
	return <UserSettingsContent username={username} />;
};

const UserSettingsContent = ({ username }: { username: string }) => {
	const { users, isLoading, authError } = useUsers();
	const { projects } = useProjects();
	const me = useMe();
	const user = users.find((u) => u.username === username);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);
	const [isAdmin, setIsAdmin] = useState(false);
	const [error, setError] = useState<string>();

	const projectTags = useMemo(() => projects.map((p) => ({ value: p.id, label: p.displayName })), [projects]);

	const isSelf = getUsername() === username;

	useEffect(() => {
		if (!user) return;
		setIsAdmin(user.role === "admin");
		setSelectedProjects(
			user.projects.map((projectId) => {
				const p = projects.find((p) => p.id === projectId);
				return { value: projectId, label: p ? p.displayName : projectId };
			}),
		);
	}, [user, projects]);

	const saveUser = (nextProjects: Tag[], nextIsAdmin: boolean) => {
		if (!user) return;
		setSelectedProjects(nextProjects);
		setIsAdmin(nextIsAdmin);
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
			.catch((err) => {
				setError(err instanceof Error ? err.message : "Failed to update user");
				createToast("Failed to update user", "error");
			});
	};

	if (authError) return <p>You don't have permission to view this page.</p>;
	if (isLoading) return <div className="loading-spinner" />;
	if (!user) return <p>User not found.</p>;

	return (
		<SettingsForm>
			<SettingsHeader title={user.username} backHref="/settings/users" backLabel="Back to users" />
			<SettingsField label="Projects" description="Projects this user can view.">
				<Tags
					selected={selectedProjects}
					suggestions={projectTags}
					onAdd={(tag) => saveUser([...selectedProjects, tag], isAdmin)}
					onDelete={(i) =>
						saveUser(
							selectedProjects.filter((_, index) => i !== index),
							isAdmin,
						)
					}
					noOptionsText="No matching projects"
				/>
			</SettingsField>
			{me.role === "admin" && (
				<SettingsSwitch
					label="Administrator access"
					description={
						<>
							Administrators can edit and create projects, entities, and users.
							{isSelf && " You cannot change your own role."}
						</>
					}
					checked={isAdmin}
					disabled={isSelf}
					onCheckedChange={(checked) => saveUser(selectedProjects, checked)}
				/>
			)}
			{me.role === "admin" && !isSelf && (
				<div className={styles.dangerZone}>
					<DeleteDialog
						id={user.username}
						displayName={user.username}
						type="user"
						onDeleted={() => {
							window.location.href = "/settings/users";
						}}
						trigger={
							<button type="button" className={styles.deleteButton}>
								Delete user
							</button>
						}
					/>
				</div>
			)}
			{error && (
				<article role="alert" className={styles.error}>
					{error}
				</article>
			)}
		</SettingsForm>
	);
};
