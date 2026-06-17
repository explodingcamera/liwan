import styles from "./snippet.module.css";

import { CopyIcon } from "lucide-react";

import { useConfig } from "../../hooks/api";
import { createToast } from "../toast";

export const Snippet = ({ entityId }: { entityId: string }) => {
	const { config } = useConfig();
	const baseUrl = config?.baseUrl ?? window.location.origin;
	const scriptUrl = `${baseUrl.replace(/\/$/, "")}/script.js`;
	const code = `<script type="module" data-entity="${entityId}" src="${scriptUrl}"></script>`;

	return (
		<div className={styles.snippet}>
			<code>
				<span className={styles.tag}>{"<script"}</span>
				{' type="module" data-entity="'}
				<span className={styles.entity}>{entityId}</span>
				{'" src="'}
				{scriptUrl}
				{`"`}
				<span className={styles.tag}>{"></script>"}</span>
			</code>
			<button
				type="button"
				className={`secondary outline ${styles.copyButton}`}
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
