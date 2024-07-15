import { id } from "date-fns/locale";
import { api, useEntities, useProjects, useQuery, useUsers } from "../../api";
import { Table, type Column } from "../table";

export const ProjectsTable = () => {
	const { projects, isLoading } = useProjects();
	const columns: Column<(typeof projects)[number]>[] = [
		{
			id: "public",
			render: (row) => (row.public ? "Public" : "Private"),
		},
		{
			id: "id",
			header: "ID",
			render: (row) => <a href={`/settings/projects/${row.id}`}>{row.id}</a>,
		},
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={`/settings/projects/${row.id}`}>{row.displayName}</a>,
		},
	];

	return <Table columns={columns} rows={projects} />;
};

export const EntitiesTable = () => {
	const { entities, isLoading } = useEntities();
	const columns: Column<(typeof entities)[number]>[] = [
		{
			id: "id",
			header: "ID",
			render: (row) => <a href={`/settings/entities/${row.id}`}>{row.id}</a>,
		},
		{
			id: "displayName",
			header: "Name",
			render: (row) => <a href={`/settings/entities/${row.id}`}>{row.displayName}</a>,
		},
	];

	return <Table columns={columns} rows={entities} />;
};

export const UsersTable = () => {
	const { users, isLoading } = useUsers();
	const rows = users.map((user) => ({ id: user.username, ...user })) ?? [];

	const columns: Column<(typeof rows)[number]>[] = [
		{
			id: "username",
			header: "Username",
			render: (row) => <a href={`/settings/users/${row.username}`}>{row.username}</a>,
		},
		{
			id: "role",
			header: "Role",
			render: (row) => row.role,
		},
	];

	return <Table columns={columns} rows={rows} />;
};
