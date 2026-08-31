import styles from "./authentication.module.css";

import { useEffect, useState } from "react";
import { SiGoogle, SiOpenid } from "@icons-pack/react-simple-icons";
import { KeyRoundIcon } from "lucide-react";

import { api } from "@/api";
import { createToast } from "@/components/ui/toast";
import type { ExternalAuthProvider, ExternalAuthSettings, ExternalAuthSettingsUpdate } from "@/constants";
import { SettingsField, SettingsForm, SettingsHeader, SettingsSwitch } from "../form";

const providers: { value: ExternalAuthProvider; label: string; description: string }[] = [
	{ value: "oidc", label: "OpenID Connect", description: "Any compatible provider" },
	{ value: "google", label: "Google", description: "Google Workspace" },
	{ value: "microsoft", label: "Microsoft Entra ID", description: "Work or school accounts" },
];

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

const errorMessage = (error: unknown) =>
	typeof error === "object" && error && "message" in error && typeof error.message === "string"
		? error.message
		: "Failed to update authentication settings";

type ProviderSettingsProps = {
	settings: ExternalAuthSettings;
	clientSecret: string;
	secretConfigured: boolean;
	update: <K extends keyof ExternalAuthSettings>(key: K, value: ExternalAuthSettings[K]) => void;
	selectProvider: (provider: ExternalAuthProvider | "internal") => void;
	setClientSecret: (value: string) => void;
};

const ProviderSettings = ({
	settings,
	clientSecret,
	secretConfigured,
	update,
	selectProvider,
	setClientSecret,
}: ProviderSettingsProps) => (
	<>
		<fieldset className={styles.providerFieldset}>
			<legend>Sign-in method</legend>
			<p>Use Liwan passwords only, or add single sign-on with one external provider.</p>
			<div className={styles.providerGrid}>
				<label className={styles.providerCard}>
					<input
						type="radio"
						name="provider"
						value="internal"
						checked={!settings.enabled}
						onChange={() => selectProvider("internal")}
					/>
					<span className={styles.providerLogo}>
						<KeyRoundIcon aria-hidden="true" />
					</span>
					<span className={styles.providerText}>
						<strong>Internal</strong>
						<small>Liwan username and password</small>
					</span>
				</label>
				{providers.map((provider) => (
					<label className={styles.providerCard} key={provider.value}>
						<input
							type="radio"
							name="provider"
							value={provider.value}
							checked={settings.enabled && settings.provider === provider.value}
							onChange={() => selectProvider(provider.value)}
						/>
						<span className={styles.providerLogo}>
							<ProviderLogo provider={provider.value} />
						</span>
						<span className={styles.providerText}>
							<strong>{provider.label}</strong>
							<small>{provider.description}</small>
						</span>
					</label>
				))}
			</div>
			{!settings.enabled && (
				<p className={styles.internalStatus}>
					Liwan username and password sign-in is active. No additional setup is required.
				</p>
			)}
		</fieldset>
		{settings.enabled && (
			<section className={styles.signInOptions} aria-labelledby="sign-in-options-heading">
				<h2 id="sign-in-options-heading">Single sign-on options</h2>
				<p>Control account creation for this provider.</p>
				<div className={styles.optionList}>
					<SettingsSwitch
						name="allowUserCreation"
						label="Allow new users"
						description="Create an account after a user's first verified sign-in."
						checked={settings.allowUserCreation}
						onCheckedChange={(allowUserCreation) => update("allowUserCreation", allowUserCreation)}
					/>
					<SettingsSwitch
						name="allowSessionReuse"
						label="Reuse provider session"
						description="Use an existing provider session instead of asking the user to sign in again."
						checked={settings.allowSessionReuse}
						onCheckedChange={(allowSessionReuse) => update("allowSessionReuse", allowSessionReuse)}
					/>
				</div>
			</section>
		)}
		{settings.enabled && (
			<div className={styles.configuration}>
				{settings.provider === "oidc" && (
					<SettingsField
						label="Sign-in button label"
						description='Appears as "Continue with [label]" on the sign-in page.'
						name="displayName"
					>
						<input
							name="displayName"
							value={settings.displayName}
							required={settings.enabled}
							onChange={(event) => update("displayName", event.currentTarget.value)}
						/>
					</SettingsField>
				)}
				<SettingsField label="Client ID" name="clientId">
					<input
						name="clientId"
						value={settings.clientId}
						required={settings.enabled}
						onChange={(event) => update("clientId", event.currentTarget.value)}
					/>
				</SettingsField>
				<SettingsField
					label="Client secret"
					description={
						secretConfigured
							? "A secret is stored. Enter a new value to replace it."
							: "Enter the client secret from your provider."
					}
					name="clientSecret"
				>
					<input
						type="password"
						name="clientSecret"
						value={clientSecret}
						placeholder={secretConfigured ? "∗∗∗∗∗∗∗∗" : undefined}
						autoComplete="new-password"
						required={settings.enabled && !secretConfigured}
						onChange={(event) => setClientSecret(event.currentTarget.value)}
					/>
				</SettingsField>
				{settings.provider === "oidc" && (
					<SettingsField
						label="Issuer URL"
						description="The base URL used to discover your provider's OpenID Connect configuration."
						name="issuerUrl"
					>
						<input
							type="url"
							name="issuerUrl"
							value={settings.issuerUrl ?? ""}
							required={settings.enabled}
							onChange={(event) => update("issuerUrl", event.currentTarget.value || null)}
						/>
					</SettingsField>
				)}
				{settings.provider === "google" && (
					<SettingsField
						label="Google Workspace domain"
						description="Optional. Only accounts managed by this Google Workspace domain can sign in."
						name="allowedDomain"
					>
						<input
							name="allowedDomain"
							value={settings.allowedDomain ?? ""}
							onChange={(event) => update("allowedDomain", event.currentTarget.value || null)}
						/>
					</SettingsField>
				)}
				{settings.provider === "microsoft" && (
					<SettingsField
						label="Tenant ID"
						description="The directory ID for the Microsoft Entra tenant that can sign in."
						name="tenantId"
					>
						<input
							name="tenantId"
							value={settings.tenantId ?? ""}
							required={settings.enabled}
							onChange={(event) => update("tenantId", event.currentTarget.value || null)}
						/>
					</SettingsField>
				)}
			</div>
		)}
	</>
);

export const AuthenticationSettingsPage = () => {
	const [settings, setSettings] = useState<ExternalAuthSettings>();
	const [savedSettings, setSavedSettings] = useState<ExternalAuthSettings>();
	const [error, setError] = useState<string>();
	const [clientSecret, setClientSecret] = useState("");

	useEffect(() => {
		api["/api/dashboard/admin/auth"]
			.get()
			.json()
			.then((settings) => {
				setSettings(settings);
				setSavedSettings(settings);
			})
			.catch((error) => setError(errorMessage(error)));
	}, []);

	if (error && !settings) return <article role="alert">{error}</article>;
	if (!settings) return <div className="loading-spinner" />;

	const update = <K extends keyof ExternalAuthSettings>(key: K, value: ExternalAuthSettings[K]) =>
		setSettings({ ...settings, [key]: value });
	const selectProvider = (provider: ExternalAuthProvider | "internal") => {
		if (provider === "internal") {
			setSettings({ ...settings, enabled: false });
			return;
		}
		if (provider === settings.provider) {
			setSettings({ ...settings, enabled: true });
			return;
		}
		setClientSecret("");
		setSettings({
			...settings,
			enabled: true,
			provider,
			displayName: provider === "oidc" ? "OpenID Connect" : settings.displayName,
			clientId: "",
			issuerUrl: null,
			allowedDomain: null,
			tenantId: null,
		});
	};

	const save = () => {
		setError(undefined);
		const displayName =
			settings.provider === "oidc"
				? settings.displayName
				: (providers.find((provider) => provider.value === settings.provider)?.label ?? settings.displayName);
		const request: ExternalAuthSettingsUpdate = {
			enabled: settings.enabled,
			provider: settings.provider,
			displayName,
			clientId: settings.clientId,
			clientSecret: clientSecret || null,
			clearClientSecret: false,
			issuerUrl: settings.provider === "oidc" ? settings.issuerUrl : null,
			allowedDomain: settings.provider === "google" ? settings.allowedDomain : null,
			tenantId: settings.provider === "microsoft" ? settings.tenantId : null,
			allowUserCreation: settings.allowUserCreation,
			allowSessionReuse: settings.allowSessionReuse,
		};

		api["/api/dashboard/admin/auth"]
			.put({ json: request })
			.json()
			.then((next) => {
				if (typeof next === "string") throw new Error(next);
				setSettings(next);
				setSavedSettings(next);
				setClientSecret("");
				createToast("Authentication settings updated", "success");
			})
			.catch((error) => {
				setError(errorMessage(error));
				createToast("Failed to update authentication settings", "error");
			});
	};

	const canKeepClientSecret = Boolean(
		savedSettings?.clientSecretConfigured &&
			settings.provider === savedSettings.provider &&
			settings.clientId.trim() === savedSettings.clientId &&
			(settings.provider !== "oidc" || settings.issuerUrl?.trim() === savedSettings.issuerUrl) &&
			(settings.provider !== "microsoft" || settings.tenantId?.trim().toLowerCase() === savedSettings.tenantId),
	);
	const secretConfigured = canKeepClientSecret;

	return (
		<div className={styles.page}>
			<SettingsHeader title="Authentication" saveForm="authentication-settings-form" />
			<SettingsForm
				id="authentication-settings-form"
				onSubmit={(event) => {
					event.preventDefault();
					save();
				}}
			>
				<ProviderSettings
					settings={settings}
					clientSecret={clientSecret}
					secretConfigured={secretConfigured}
					update={update}
					selectProvider={selectProvider}
					setClientSecret={setClientSecret}
				/>
				{settings.enabled && (
					<div className={styles.callbackSection}>
						<h2>Callback URL</h2>
						<p>Add this exact URL to the provider application's allowed redirect URLs.</p>
						<code className={styles.callback}>{settings.callbackUrl}</code>
					</div>
				)}
				{error && <article role="alert">{error}</article>}
			</SettingsForm>
		</div>
	);
};
