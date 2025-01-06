import { Fragment, type ReactElement, useEffect, useRef, useState } from "react";
import styles from "./tables.module.css";

import { EditIcon, EllipsisVerticalIcon, RectangleEllipsisIcon, TrashIcon } from "lucide-react";
import { type Column, Table } from "../table";
import { DeleteDialog, EditEntity, EditPassword, EditProject, EditUser } from "./dialogs";

import {
	type EntityResponse,
	type ProjectResponse,
	type UserResponse,
	useEntities,
	useMe,
	useProjects,
	useUsers,
} from "../../api";
import { cls } from "../../utils";
import { createToast } from "../toast";

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

const ProjectDropdown = ({ project }: { project: ProjectResponse }) => {
	const { role } = useMe();
	if (role === "user") {
		return null;
	}

	const options: DropdownOptions = {
		edit: (close) => (
			<EditProject
				project={project}
				trigger={
					<button type="button" onClick={close}>
						<EditIcon size={18} />
						Edit
					</button>
				}
			/>
		),
		delete: (close) => (
			<DeleteDialog
				id={project.id}
				displayName={project.displayName}
				type="project"
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

export const ProjectsTable = () => {
	const { projects, isLoading } = useProjects();
	const columns: Column<(typeof projects)[number]>[] = [
		{
			id: "displayName",
			header: "Name",
			render: (row) => <span>{row.displayName}</span>,
			nowrap: true,
		},
		{
			id: "id",
			header: "ID",
			render: (row) => <i>{row.id}</i>,
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
							<u data-tooltip={`ID: ${entity.id}`}>{entity.displayName}</u>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => <ProjectDropdown project={row} />,
		},
	];

	return <Table columns={columns} rows={projects} isLoading={isLoading} />;
};

const EntityDropdown = ({ entity }: { entity: EntityResponse }) => {
	const { role } = useMe();
	if (role === "user") {
		return null;
	}

	const options: DropdownOptions = {
		copy: (close) => (
			<button
				type="button"
				onClick={() => {
					navigator.clipboard
						.writeText(
							`<script type="module" data-entity="${entity.id}" src="${window.location.origin}/script.js"></script>`,
						)
						.then(() => createToast("Snippet copied to clipboard", "info"))
						.catch(() => createToast("Failed to copy snippet to clipboard", "error"));

					close();
				}}
			>
				<RectangleEllipsisIcon size={18} />
				Copy Snippet
			</button>
		),
		edit: (close) => (
			<EditEntity
				entity={entity}
				trigger={
					<button type="button" onClick={close}>
						<EditIcon size={18} />
						Edit
					</button>
				}
			/>
		),
		delete: (close) => (
			<DeleteDialog
				id={entity.id}
				displayName={entity.displayName}
				type="entity"
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

export const EntitiesTable = () => {
	const { entities, isLoading } = useEntities();

	if (!useAdminPerms()) {
		return <>You don't have permission to view this page.</>;
	}

	const columns: Column<(typeof entities)[number]>[] = [
		{
			id: "displayName",
			// icon: <TagIcon size={18} />,
			header: "Name",
			render: (row) => <span>{row.displayName}</span>,
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
							<u data-tooltip={`ID: ${project.id}`}>{project.displayName}</u>
						</Fragment>
					))}
				</>
			),
			full: true,
		},
		{
			id: "edit",
			render: (row) => <EntityDropdown entity={row} />,
		},
	];

	return <Table columns={columns} rows={entities} isLoading={isLoading} />;
};

const UserDropdown = ({ user }: { user: UserResponse }) => {
	const { username } = useMe();
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

const useAdminPerms = () => {
	const { role } = useMe();
	const [isMounted, setIsMounted] = useState(false);
	useEffect(() => {
		setIsMounted(true);
	});
	return !isMounted || role === "admin";
};

export const UsersTable = () => {
	const { users, isLoading } = useUsers();
	const rows = users.map((user) => ({ id: user.username, ...user })) ?? [];

	if (!useAdminPerms()) {
		return <>You don't have permission to view this page.</>;
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
