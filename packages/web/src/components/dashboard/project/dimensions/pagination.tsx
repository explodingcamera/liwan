import styles from "./dimensions.module.css";

import { useState } from "react";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";

import type { ProjectQuery } from "..";

const pageSize = 6;

export const useReportPagination = <Row,>(rows: readonly Row[] | undefined, query: ProjectQuery, report?: string) => {
	const scope = JSON.stringify([
		query.project.id,
		query.range.cacheKey(),
		query.comparisonRange?.cacheKey(),
		query.metric,
		query.eventName,
		query.filters,
		report,
	]);
	const [selection, setSelection] = useState({ scope, page: 0 });
	const lastPage = Math.max(0, Math.ceil((rows?.length ?? 0) / pageSize) - 1);
	const page = selection.scope === scope ? Math.min(selection.page, lastPage) : 0;
	if (selection.scope !== scope) setSelection({ scope, page: 0 });

	return {
		pageRows: rows?.slice(page * pageSize, (page + 1) * pageSize),
		hasPages: lastPage > 0,
		previous: page > 0 ? () => setSelection({ scope, page: page - 1 }) : undefined,
		next: page < lastPage ? () => setSelection({ scope, page: page + 1 }) : undefined,
	};
};

export const ReportPagination = ({ previous, next }: { previous?: () => void; next?: () => void }) => (
	<div className={styles.pagination}>
		{previous && (
			<button type="button" aria-label="Previous page" onClick={previous}>
				<ChevronLeftIcon size={16} />
			</button>
		)}
		<button type="button" aria-label="Next page" onClick={next} disabled={!next}>
			<ChevronRightIcon size={16} />
		</button>
	</div>
);
