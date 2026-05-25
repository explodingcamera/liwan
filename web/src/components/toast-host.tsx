import { Toast } from "@base-ui/react/toast";
import { XIcon } from "lucide-react";
import { toastManager } from "./toast";
import styles from "./toast.module.css";

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
