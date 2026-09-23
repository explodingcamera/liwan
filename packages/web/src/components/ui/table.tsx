import styles from "./table.module.css";

import type { ReactElement } from "react";
import { useEffect, useState } from "react";

import { LoadingSpinner } from "./loading";

export type Column<T> = {
	id: string;
	header?: string | ReactElement;
	icon?: ReactElement;
	render?: (row: T) => ReactElement | string;
	full?: boolean;
	nowrap?: boolean;
};

export const Table = <T extends { id: string }>({
	rows,
	columns,
	isLoading,
	emptyMessage = "No results.",
}: {
	rows: T[];
	columns: Column<T>[];
	isLoading: boolean;
	emptyMessage?: string;
}) => {
	// prevent hydration mismatch
	const [loading, setLoading] = useState(true);
	useEffect(() => setLoading(isLoading), [isLoading]);

	if (loading) return <LoadingSpinner />;

	return (
		<div className={styles.container}>
			<table className={styles.table}>
				<thead>
					<tr>
						{columns?.map((col) => (
							<th
								scope="col"
								key={col.id}
								className={
									[col.full && styles.full, col.nowrap && styles.nowrap].filter(Boolean).join(" ") || undefined
								}
							>
								{col.icon ? (
									<div className={styles.icon}>
										{col.icon}
										{col.header ?? null}
									</div>
								) : (
									(col.header ?? null)
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
								{emptyMessage}
							</td>
						</tr>
					)}
				</tbody>
			</table>
		</div>
	);
};
