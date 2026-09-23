import styles from "./loading.module.css";

import { cls } from "@/utils";

/** A loading indicator that stays hidden for fast operations. */
export const LoadingSpinner = ({ className }: { className?: string }) => (
	<div className={cls("loading-spinner", styles.spinner, className)} aria-hidden="true" />
);
