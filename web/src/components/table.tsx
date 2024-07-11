export type Column<T> = {
	id: string;
	header?: string | JSX.Element;
	render?: (row: T) => JSX.Element | string;
};

export const Table = <T extends { id: string }>({
	rows,
	columns,
}: {
	rows: T[];
	columns: Column<T>[];
}) => {
	return (
		<div>
			<div>
				<table className="striped">
					<thead>
						<tr>
							{columns?.map((col) => (
								<td key={col.id}>{col.header ?? null}</td>
							))}
						</tr>
					</thead>
					<tbody>
						{rows?.length ? (
							rows.map((row) => (
								<tr key={row.id}>
									{columns?.map((col) => (
										<td key={col.id}>{col.render ? col.render(row) : null}</td>
									))}
								</tr>
							))
						) : (
							<tr>
								<td colSpan={columns?.length} className="h-24 text-center">
									No results.
								</td>
							</tr>
						)}
					</tbody>
				</table>
			</div>
		</div>
	);
};
