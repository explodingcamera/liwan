import { id } from "date-fns/locale";
import { api, useQuery } from "../../api";
import { Table, type Column } from "../table";

export const ProjectsTable = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["projects"],
		queryFn: () => api["/api/dashboard/projects"].get().json(),
	});

	const rows = Object.entries(data?.projects ?? {}).map(([id, rest]) => ({ id, ...rest })) ?? [];

	const columns: Column<(typeof rows)[number]>[] = [
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

	return <Table columns={columns} rows={rows} />;
};

export const EntitiesTable = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["entities"],
		queryFn: () => api["/api/dashboard/entities"].get().json(),
	});

	const rows = Object.entries(data ?? {}).map(([id, displayName]) => ({ id, displayName })) ?? [];

	const columns: Column<(typeof rows)[number]>[] = [
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

	return <Table columns={columns} rows={rows} />;
};

export const UsersTable = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["users"],
		queryFn: () => api["/api/dashboard/users"].get().json(),
	});

	const rows = data?.map((user) => ({ id: user.username, ...user })) ?? [];

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
