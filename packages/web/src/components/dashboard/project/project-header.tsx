import cardStyles from "./card.module.css";
import styles from "./project-header.module.css";

import { CircleIcon, LockIcon, LockOpenIcon } from "lucide-react";

import { appPath } from "@/config";
import type { ProjectResponse, StatsResponse } from "@/constants";
import { formatMetricVal } from "./metric-format";

export const ProjectHeader = ({
	project,
	stats,
	isUpdating,
}: {
	stats?: StatsResponse;
	project: ProjectResponse;
	isUpdating: boolean;
}) => {
	const count = stats?.currentVisitors;
	return (
		<h1 className={styles.statsHeader}>
			<a href={appPath(`/p/${project.id}`)} className={cardStyles.card}>
				{project.visibility === "unlisted" ? (
					<LockOpenIcon size={16} />
				) : (
					project.visibility !== "public" && <LockIcon size={16} />
				)}
				<span title={project.displayName}>{project.displayName}</span>
			</a>
			{count !== undefined && (
				<span className={styles.online} data-updating={isUpdating}>
					<CircleIcon fill="var(--success)" color="var(--success)" size={10} />
					<CircleIcon fill="var(--success)" color="var(--success)" size={10} className={styles.pulse} />
					{formatMetricVal(count, "unique_visitors")} {count === 1 ? "Current Visitor" : "Current Visitors"}
				</span>
			)}
		</h1>
	);
};
