import { CopyIcon } from "lucide-react";
import { createToast } from "../toast";
import styles from "./snippet.module.css";

export const Snippet = ({ entityId }: { entityId: string }) => {
	const code = `<script type="module" data-entity="${entityId}" src="${window.location.origin}/script.js"></script>`;

	return (
		<div className={styles.snippet}>
			<code>
				<span className={styles.tag}>{"<script"}</span>
				{' type="module" data-entity="'}
				<span className={styles.entity}>{entityId}</span>
				{'" src="'}
				{window.location.origin}/script.js{`"`}
				<span className={styles.tag}>{"></script>"}</span>
			</code>
			<button
				type="button"
				className="secondary outline"
				aria-label="Copy snippet"
				onClick={() =>
					navigator.clipboard
						.writeText(code)
						.then(() => createToast("Snippet copied to clipboard", "info"))
						.catch(() => createToast("Failed to copy snippet to clipboard", "error"))
				}
			>
				<CopyIcon size={16} />
			</button>
		</div>
	);
};
