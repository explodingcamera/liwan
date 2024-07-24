import { useEffect, useState } from "react";
import styles from "./project.module.css";

import { api, useQuery } from "../api";
const server = typeof window === "undefined";

export const Project = () => {
	const [projectId, setProjectId] = useState<string | null>(null);
	useEffect(() => {
		if (server) return;
		setProjectId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);

	const { data, isLoading, error } = useQuery({
		enabled: projectId !== null,
		queryKey: ["project", projectId],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}"].get({ params: { project_id: projectId as string } }).json(),
	});
 
	return <div className={styles.project}>{data?.displayName}</div>;
};
