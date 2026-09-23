import styles from "./toast.module.css";

import { Toast } from "@base-ui/react/toast";
import { XIcon } from "lucide-react";

export type ToastType = "success" | "error" | "info" | "warning";
export type ToastData = { tone: ToastType };

export const toastManager = Toast.createToastManager<ToastData>();

const toastTitles: Record<ToastType, string> = {
	success: "Success",
	error: "Error",
	info: "Info",
	warning: "Warning",
};

export const createToast = (message: string, type: ToastType = "info") => {
	toastManager.add({
		title: toastTitles[type],
		description: message,
		type,
		data: { tone: type },
		timeout: 3200,
		priority: type === "error" ? "high" : "low",
	});
};

export const ToastHost = () => (
	<Toast.Provider toastManager={toastManager} timeout={3200} limit={4}>
		<Toast.Portal>
			<Toast.Viewport className={styles.viewport}>
				<ToastList />
			</Toast.Viewport>
		</Toast.Portal>
	</Toast.Provider>
);

const ToastList = () => {
	const { toasts } = Toast.useToastManager();

	return toasts.map((toast) => (
		<Toast.Root key={toast.id} toast={toast} className={styles.toast} swipeDirection={["right", "down"]}>
			<Toast.Content className={styles.content}>
				<span className={styles.indicator} aria-hidden />
				<div className={styles.text}>
					<Toast.Title className={styles.title} />
					<Toast.Description className={styles.description} />
				</div>
				<Toast.Close className={styles.close} aria-label="Dismiss notification">
					<XIcon size={16} />
				</Toast.Close>
			</Toast.Content>
		</Toast.Root>
	));
};
