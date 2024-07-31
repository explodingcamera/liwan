import styles from "./table.module.css";

export type Column<T> = {
	id: string;
	header?: string | JSX.Element;
	icon?: JSX.Element;
	render?: (row: T) => JSX.Element | string;
	full?: boolean;
	nowrap?: boolean;
};

export const Table = <T extends { id: string }>({
	rows,
	columns,
}: {
	rows: T[];
	columns: Column<T>[];
}) => {
	return (
		<div className="overflow-auto">
			<table className={styles.table}>
				<thead>
					<tr>
						{columns?.map((col) => (
							<th scope="col" key={col.id} className={col.full ? styles.full : undefined}>
								{col.icon ? (
									<div className={styles.icon}>
										{col.icon}
										{col.header ?? null}
									</div>
								) : (
									col.header ?? null
								)}
							</th>
						))}
					</tr>
				</thead>
				<tbody>
					{rows?.length ? (
						rows.map((row) => (
							<tr key={row.id}>
								{columns?.map((col) => (
									<td key={col.id} className={col.nowrap ? styles.nowrap : undefined}>
										{col.render ? col.render(row) : null}
									</td>
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
	);
};
