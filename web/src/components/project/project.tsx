import { CircleIcon, LockIcon } from "lucide-react";
import styles from "./project.module.css";

import type { ProjectResponse, StatsResponse } from "../../api";
import { formatMetricVal } from "../../utils";
import { CardLink } from "../card";

export const ProjectHeader = ({ project, stats }: { stats?: StatsResponse; project: ProjectResponse }) => {
	return (
		<h1 className={styles.statsHeader}>
			<span>
				<CardLink href={`/p/${project.id}`}>
					{project.public ? null : <LockIcon size={16} />}
					{project.displayName}
				</CardLink>
				&nbsp;
			</span>
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
