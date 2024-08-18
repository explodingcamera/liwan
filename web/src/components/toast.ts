import styles from "./toast.module.css";

type ToastType = "success" | "error" | "info" | "warning";

export const createToast = (message: string, type: ToastType = "info") => {
	let toastContainer = document.getElementById("toast-container");
	if (!toastContainer) {
		toastContainer = document.createElement("div");
		toastContainer.id = "toast-container";
		toastContainer.className = styles.toastContainer;
		toastContainer.setAttribute("role", "alert");
		toastContainer.setAttribute("aria-live", "assertive");
		toastContainer.setAttribute("aria-atomic", "true");
		document.body.appendChild(toastContainer);
	}

	const toast = document.createElement("div");
	toast.className = `${styles.toast} ${styles[type]}`;
	toast.textContent = message;

	toastContainer.appendChild(toast);
	toast.animate(
		[
			{ opacity: 0, transform: "translateY(0.5rem)" },
			{ opacity: 1, transform: "translateY(0)" },
		],
		{
			duration: 300,
			easing: "ease-out",
		},
	);

	setTimeout(() => {
		const fadeOut = toast.animate(
			[
				{ opacity: 1, transform: "translateY(0)" },
				{ opacity: 0, transform: "translateY(0.5rem)" },
			],
			{
				duration: 500,
				easing: "ease-out",
			},
		);

		fadeOut.onfinish = () => {
			toast.remove();
			if (toastContainer && toastContainer.childElementCount === 0) {
				toastContainer.remove();
			}
		};
	}, 2500);
};
