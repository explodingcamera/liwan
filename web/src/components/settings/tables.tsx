import { Fragment, type ReactElement, useRef } from "react";
import styles from "./tables.module.css";

import { EditIcon, EllipsisVerticalIcon, RectangleEllipsisIcon, SettingsIcon, TrashIcon } from "lucide-react";
import { type Column, Table } from "../table";
import { DeleteDialog, EditPassword, EditUser } from "./dialogs";

import { type UserResponse, useEntities, useMe, useProjects, useUsers } from "../../api";
import { cls, getUsername } from "../../utils";

type DropdownOptions = Record<string, ((close: () => void) => ReactElement) | null>;

const Dropdown = ({ options }: { options: DropdownOptions }) => {
	const detailsRef = useRef<HTMLDetailsElement>(null);
	return (
		<details className={cls("dropdown", styles.edit)} ref={detailsRef}>
			<summary>
				<EllipsisVerticalIcon />
			</summary>
			<ul>
				{Object.entries(options)
					.filter(([, val]) => val !== null)
					.map(([key, val]) => (
						<li key={key}>{val?.(() => detailsRef.current?.removeAttribute("open"))}</li>
					))}
			</ul>
		</details>
	);
};

const SettingsLink = ({ href, label }: { href: string; label: string }) => {
	const { role } = useMe();
	if (role === "user") {
		return null;
	}

	return (
		<a href={href} className={styles.settingsLink} aria-label={label} title={label}>
			<SettingsIcon size={18} />
		</a>
	);
};

export const ProjectsTable = () => {
	const { projects, isLoading } = useProjects();

	const columns: Column<(typeof projects)[number]>[] = [
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={`/settings/projects/${row.id}`}>{row.displayName}</a>,
			nowrap: true,
		},
		{
			id: "public",
			header: "Visibility",
			render: (row) => <>{row.public ? "Public" : "Private"}</>,
		},
		{
			id: "entities",
			header: "Entities",
			render: (row) => (
				<>
					{row.entities.map((entity, i) => (
						<Fragment key={entity.id}>
							{i > 0 && ", "}
							<a href={`/settings/entities/${entity.id}`}>{entity.displayName}</a>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={`/settings/projects/${row.id}`} label={`Open ${row.displayName} settings`} />
			),
		},
	];

	return <Table columns={columns} rows={projects} isLoading={isLoading} />;
};

export const EntitiesTable = () => {
	const { entities, isLoading, authError } = useEntities();

	if (authError) {
		return "You don't have permission to view this page.";
	}

	const columns: Column<(typeof entities)[number]>[] = [
		{
			id: "displayName",
			// icon: <TagIcon size={18} />,
			header: "Name",
			render: (row) => <a href={`/settings/entities/${row.id}`}>{row.displayName}</a>,
			nowrap: true,
		},
		{
			id: "id",
			// icon: <WholeWordIcon size={18} />,
			header: "ID",
			render: (row) => <i>{row.id}</i>,
			nowrap: true,
		},
		{
			id: "projects",
			// icon: <AppWindowIcon size={18} />,
			header: "Projects",
			render: (row) => (
				<>
					{row.projects.map((project, i) => (
						<Fragment key={project.id}>
							{i > 0 && ", "}
							<a href={`/settings/projects/${project.id}`}>{project.displayName}</a>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => (
				<SettingsLink href={`/settings/entities/${row.id}`} label={`Open ${row.displayName} settings`} />
			),
		},
	];

	return <Table columns={columns} rows={entities} isLoading={isLoading} />;
};

const UserDropdown = ({ user }: { user: UserResponse }) => {
	const username = getUsername();
	const options: DropdownOptions = {
		edit:
			username !== user.username
				? (close) => (
						<EditUser
							trigger={
								<button type="button" onClick={close}>
									<EditIcon size={18} />
									Edit
								</button>
							}
							user={user}
						/>
					)
				: null,
		updatePassword: (close) => (
			<EditPassword
				user={user}
				trigger={
					<button type="button" onClick={close}>
						<RectangleEllipsisIcon size={18} />
						Update Password
					</button>
				}
			/>
		),
		delete: (close) => (
			<DeleteDialog
				id={user.username}
				displayName={user.username}
				type="user"
				trigger={
					<button type="button" onClick={close} className={styles.danger}>
						<TrashIcon size={18} />
						Delete
					</button>
				}
			/>
		),
	};

	return <Dropdown options={options} />;
};

export const UsersTable = () => {
	const { users, isLoading, authError } = useUsers();
	const rows = users.map((user) => ({ id: user.username, ...user })) ?? [];

	// TODO: if the user isn't an admin, show no perms
	if (authError) {
		return "You don't have permission to view this page.";
	}

	const columns: Column<(typeof rows)[number]>[] = [
		{
			id: "username",
			header: "Username",
			// icon: <UserIcon size={18} />,
			render: (row) => <span>{row.username}</span>,
			nowrap: true,
		},
		{
			id: "role",
			header: "Role",
			// icon: <ShieldIcon size={18} />,
			render: (row) => row.role,
			full: true,
		},
		{
			id: "edit",
			render: (row) => <UserDropdown user={row} />,
		},
	];

	return <Table columns={columns} rows={rows} isLoading={isLoading} />;
};
