import { useEffect, useState } from "react";
import styles from "./project.module.css";

const server = typeof window === "undefined";

export const Project = () => {
	const [projectId, setProjectId] = useState<string | null>(null);
	useEffect(() => {
		if (server) return;
		setProjectId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);
	const loading = projectId === null;
	if (loading) return null;
	return <div className={styles.project} />;
};
