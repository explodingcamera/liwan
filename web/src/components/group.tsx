import { useEffect, useState } from "react";
import styles from "./group.module.css";
import type * as api from "../api";

const dummyGroup: api.Group = {
	displayName: "Personal Websites",
	entities: {
		portfolio: "Portfolio",
		blog: "Blog",
	},
};

const server = typeof window === "undefined";

export const Group = () => {
	const [groupId, setGroupId] = useState<string | null>(null);
	useEffect(() => {
		if (server) return;
		setGroupId(window?.document.location.pathname.split("/").pop() ?? null);
	}, []);
	const currentGroup = dummyGroup;
	const loading = groupId === null;
	if (loading) return null;

	return (
		<div className={styles.group}>
			<h1>{currentGroup.displayName}</h1>
		</div>
	);
};
