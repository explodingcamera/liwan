import styles from "./login.module.css";

import type { SubmitEvent } from "react";
import { useEffect, useState } from "react";
import { SiGoogle, SiOpenid } from "@icons-pack/react-simple-icons";

import { api } from "@/api/client";
import { queryClient } from "@/api/query";
import type { ExternalAuthMetadata, ExternalAuthProvider } from "@/constants";

const getReturnTo = () => {
	if (typeof window === "undefined") return "/";
	const requested = new URLSearchParams(window.location.search).get("returnTo") ?? "/";
	if (!requested.startsWith("/") || !URL.canParse(requested, window.location.origin)) return "/";

	const url = new URL(requested, window.location.origin);
	return url.origin === window.location.origin ? `${url.pathname}${url.search}${url.hash}` : "/";
};

const MicrosoftLogo = () => (
	<svg viewBox="0 0 24 24" aria-hidden="true">
		<path fill="#f25022" d="M1 1h10v10H1z" />
		<path fill="#7fba00" d="M13 1h10v10H13z" />
		<path fill="#00a4ef" d="M1 13h10v10H1z" />
		<path fill="#ffb900" d="M13 13h10v10H13z" />
	</svg>
);

const ProviderLogo = ({ provider }: { provider: ExternalAuthProvider }) => {
	if (provider === "google") return <SiGoogle color="#4285f4" aria-hidden="true" />;
	if (provider === "microsoft") return <MicrosoftLogo />;
	return <SiOpenid color="#f78c40" aria-hidden="true" />;
};

const providerName = (metadata: ExternalAuthMetadata) => {
	if (metadata.provider === "google") return "Google";
	if (metadata.provider === "microsoft") return "Microsoft";
	return metadata.displayName;
};

export const LoginPage = () => {
	const [externalAuth, setExternalAuth] = useState<ExternalAuthMetadata>();
	const [error, setError] = useState<string>();
	const [isSubmitting, setIsSubmitting] = useState(false);

	useEffect(() => {
		if (new URLSearchParams(window.location.search).get("externalAuthError") === "1") {
			setError("External sign-in failed. Please try again.");
		}

		let active = true;
		api["/api/dashboard/auth/external"]
			.get()
			.json()
			.then((metadata) => {
				if (active && metadata.enabled) setExternalAuth(metadata);
			})
			.catch(() => {});
		return () => {
			active = false;
		};
	}, []);

	const handleSubmit = async (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		setError(undefined);
		setIsSubmitting(true);

		const { username, password } = Object.fromEntries(new FormData(event.currentTarget)) as {
			username: string;
			password: string;
		};

		try {
			await api["/api/dashboard/auth/login"].post({ json: { username, password } });
			queryClient.clear();
			window.location.href = getReturnTo();
		} catch {
			setError("Invalid username or password");
			setIsSubmitting(false);
		}
	};

	return (
		<section className={`container ${styles.page}`}>
			<h1>Sign in</h1>
			{externalAuth && (
				<div>
					<a
						role="button"
						className={`secondary ${styles.externalAuth}`}
						href={`/api/dashboard/auth/external/start?${new URLSearchParams({ returnTo: getReturnTo() })}`}
					>
						<span className={styles.providerIcon}>
							<ProviderLogo provider={externalAuth.provider} />
						</span>
						Continue with {providerName(externalAuth)}
					</a>
					<div className={styles.separator}>or</div>
				</div>
			)}
			<form className={styles.form} onSubmit={handleSubmit}>
				<input
					type="text"
					name="username"
					placeholder="Username"
					aria-label="Username"
					autoComplete="username"
					required
				/>
				<input
					type="password"
					name="password"
					placeholder="Password"
					aria-label="Password"
					autoComplete="current-password"
					required
				/>
				<button type="submit" disabled={isSubmitting}>
					{isSubmitting ? "Signing in..." : "Login"}
				</button>
				{error && (
					<article className={styles.error} role="alert" aria-live="assertive">
						{error}
					</article>
				)}
			</form>
		</section>
	);
};
