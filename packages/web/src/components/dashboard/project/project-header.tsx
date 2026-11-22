import cardStyles from "./card.module.css";
import styles from "./project-header.module.css";

import { useState } from "react";
import { LockIcon, LockOpenIcon } from "lucide-react";

import { appPath } from "@/config";
import type { DimensionFilter, DimensionTableRow, ProjectResponse } from "@/constants";
import { useCurrentVisitors } from "@/hooks/api";
import { DimensionRow } from "./dimensions";
import { ReportDetailsDialog } from "./dimensions/modal";
import { formatMetricVal } from "./metric-format";

export const ProjectHeader = ({
	project,
	liveCount,
	fadeTitle = false,
	filters = [],
	onSelectPage,
}: {
	liveCount?: number;
	project: ProjectResponse;
	fadeTitle?: boolean;
	filters?: DimensionFilter[];
	onSelectPage?: (row: DimensionTableRow) => void;
}) => {
	const [fadeLiveVisitors] = useState(() => liveCount === undefined);
	const [liveOpen, setLiveOpen] = useState(false);
	const canShowPages = !project.hiddenDimensions.includes("url");
	const {
		data: live,
		isLoading,
		error,
	} = useCurrentVisitors(project.id, { pages: true, enabled: canShowPages && liveOpen });
	const count = (liveOpen ? live?.count : undefined) ?? liveCount ?? 0;
	const rows = live?.pages?.map(({ page, visitors }) => ({ dimensionValue: page, value: visitors }));
	const biggest = rows?.[0]?.value ?? 0;
	const liveLabel = (
		<span data-fade-in={fadeLiveVisitors || undefined}>
			<span className={styles.liveDot} aria-hidden="true" />
			{formatMetricVal(count, "unique_visitors")} {count === 1 ? "Live Visitor" : "Live Visitors"}
		</span>
	);

	return (
		<h1 className={styles.statsHeader}>
			<a href={appPath(`/p/${project.id}`)} className={cardStyles.card} data-fade-in={fadeTitle || undefined}>
				{project.visibility === "unlisted" ? (
					<LockOpenIcon size={16} />
				) : (
					project.visibility !== "public" && <LockIcon size={16} />
				)}
				<span title={project.displayName}>{project.displayName}</span>
			</a>
			{!canShowPages && count > 0 && <span className={styles.online}>{liveLabel}</span>}
			{canShowPages && (count > 0 || liveOpen) && (
				<ReportDetailsDialog
					title="Pages"
					dialogTitle="Live visitors by page"
					description="Last page viewed by each visitor in the past 5 minutes."
					metric="Live visitors"
					data={rows}
					isLoading={isLoading}
					error={error}
					emptyMessage="No pageviews in the past 5 minutes."
					open={liveOpen}
					onOpenChange={setLiveOpen}
					trigger={
						count > 0 && (
							<button type="button" className={styles.online}>
								{liveLabel}
							</button>
						)
					}
					renderRow={(row) => (
						<DimensionRow
							key={row.dimensionValue}
							row={row}
							biggest={biggest}
							dimension="url"
							query={{ filters, metric: "unique_visitors" }}
							onSelect={onSelectPage}
						/>
					)}
				/>
			)}
		</h1>
	);
};
