import styles from "./loading.module.css";

import { cls } from "@/utils";

/** A loading indicator that stays hidden for fast operations. */
export const LoadingSpinner = ({ immediate = false }: { immediate?: boolean }) => (
	<div className={cls(styles.spinner, immediate && styles.immediate)} role="status" aria-label="Loading" />
);
