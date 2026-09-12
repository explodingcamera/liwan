import styles from "./snippet.module.css";

import { CopyIcon } from "lucide-react";

import { runtimeConfig } from "@/config";
import { createToast } from "./toast";

const CopyButton = ({ value, label }: { value: string; label: string }) => (
	<button
		type="button"
		className={styles.copyButton}
		aria-label={`Copy ${label.toLowerCase()}`}
		onClick={() =>
			navigator.clipboard
				.writeText(value)
				.then(() => createToast(`${label} copied to clipboard`, "info"))
				.catch(() => createToast(`Failed to copy ${label.toLowerCase()} to clipboard`, "error"))
		}
	>
		<CopyIcon size={16} />
	</button>
);

export const CopyableValue = ({ value, label }: { value: string; label: string }) => (
	<div className={styles.snippet}>
		<input
			className={styles.value}
			aria-label={label}
			value={value}
			readOnly
			onFocus={(event) => event.currentTarget.select()}
		/>
		<CopyButton value={value} label={label} />
	</div>
);

export const Snippet = ({ entityId }: { entityId: string }) => {
	const baseUrl = runtimeConfig?.baseUrl ?? window.location.origin;
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
			<CopyButton value={code} label="Snippet" />
		</div>
	);
};
