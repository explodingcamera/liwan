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
