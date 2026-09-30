import cardStyles from "./card.module.css";
import styles from "./project-header.module.css";

import { CircleIcon, LockIcon, LockOpenIcon } from "lucide-react";

import { appPath } from "@/config";
import type { ProjectResponse, StatsResponse } from "@/constants";
import { formatMetricVal } from "@/utils";

export const ProjectHeader = ({ project, stats }: { stats?: StatsResponse; project: ProjectResponse }) => {
	return (
		<h1 className={styles.statsHeader}>
			<a href={appPath(`/p/${project.id}`)} className={cardStyles.card}>
				{project.visibility === "unlisted" ? (
					<LockOpenIcon size={16} />
				) : (
					project.visibility !== "public" && <LockIcon size={16} />
				)}
				{project.displayName}
			</a>
			{stats && <LiveVisitorCount count={stats.currentVisitors} />}
		</h1>
	);
};

export const LiveVisitorCount = ({ count }: { count: number }) => {
	return (
		<span className={styles.online}>
			<CircleIcon fill="#22c55e" color="#22c55e" size={10} />
			<CircleIcon fill="#22c55e" color="#22c55e" size={10} className={styles.pulse} />
			{formatMetricVal(count, "unique_visitors")} {count === 1 ? "Current Visitor" : "Current Visitors"}
		</span>
	);
};
