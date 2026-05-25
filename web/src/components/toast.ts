import { Toast } from "@base-ui/react/toast";

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
