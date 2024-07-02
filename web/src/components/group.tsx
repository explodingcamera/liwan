import { useEffect, useState } from "react";
import styles from "./group.module.css";
import type * as api from "../api";

const server = typeof window === "undefined";

export const Group = () => {
	const [groupId, setGroupId] = useState<string | null>(null);
	useEffect(() => {
		if (server) return;
		setGroupId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);
	const loading = groupId === null;
	if (loading) return null;
	return <div className={styles.group} />;
};
