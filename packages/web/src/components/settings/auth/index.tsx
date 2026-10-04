import styles from "./authentication.module.css";

import { useEffect, useRef, useState } from "react";
import { KeyRoundIcon, PlusIcon, Trash2Icon } from "lucide-react";

import { api } from "@/api";
import { ProviderLogo } from "@/components/login";
import { LoadingSpinner } from "@/components/ui/loading";
import { CopyableValue } from "@/components/ui/snippet";
import { createToast } from "@/components/ui/toast";
import type { ExternalAuthProvider, ExternalAuthSettings, ExternalAuthSettingsUpdate } from "@/constants";
import { useTeams } from "@/hooks/api";
import { SettingsField, SettingsForm, SettingsHeader, SettingsSwitch } from "../form";

const providers: { value: ExternalAuthProvider; label: string; description: string }[] = [
	{ value: "oidc", label: "OpenID Connect", description: "Any compatible provider" },
	{ value: "google", label: "Google", description: "Google Workspace" },
	{ value: "microsoft", label: "Microsoft Entra ID", description: "Work or school accounts" },
];

const errorMessage = (error: unknown) =>
	typeof error === "object" && error && "message" in error && typeof error.message === "string"
		? error.message
		: "Failed to update authentication settings";

const withTeamDefaults = (settings: ExternalAuthSettings): ExternalAuthSettings => ({
	...settings,
	defaultTeamId: settings.defaultTeamId ?? null,
	groupTeamMappings: settings.groupTeamMappings ?? [],
	additionalScopes: settings.additionalScopes ?? "",
	groupClaimName: settings.groupClaimName ?? "",
});

type ProviderSettingsProps = {
	settings: ExternalAuthSettings;
	clientSecret: string;
	secretConfigured: boolean;
	update: <K extends keyof ExternalAuthSettings>(key: K, value: ExternalAuthSettings[K]) => void;
	selectProvider: (provider: ExternalAuthProvider | "internal") => void;
	setClientSecret: (value: string) => void;
	teams: { id: string; displayName: string }[];
};

const ProviderSettings = ({
	settings,
	clientSecret,
	secretConfigured,
	update,
	selectProvider,
	setClientSecret,
	teams,
}: ProviderSettingsProps) => (
	<>
		<fieldset className={styles.providerFieldset}>
			<legend>Sign-in method</legend>
			<p>Use liwan passwords alone or alongside one external provider.</p>
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
						<strong>Internal only</strong>
						<small>No external sign-in</small>
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
				<p className={styles.internalStatus}>Users sign in with their liwan username and password.</p>
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
						description="Create an account when a user signs in successfully for the first time."
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
			<section
				className={styles.configuration}
				aria-labelledby={settings.provider === "google" ? undefined : "team-assignment-heading"}
				aria-label={settings.provider === "google" ? "Team assignment" : undefined}
			>
				{settings.provider !== "google" && (
					<div className={styles.groupMappings}>
						<h2 id="team-assignment-heading">Team assignment</h2>
						<p>Assign teams once when a new account is created.</p>
					</div>
				)}
				<SettingsField label={settings.provider === "google" ? "Default team for new users" : "Default team"}>
					<select
						value={settings.defaultTeamId ?? ""}
						onChange={(event) => update("defaultTeamId", event.currentTarget.value || null)}
					>
						<option value="">No default team</option>
						{teams.map((team) => (
							<option value={team.id} key={team.id}>
								{team.displayName}
							</option>
						))}
					</select>
				</SettingsField>
				{settings.provider === "oidc" && (
					<SettingsField
						label={settings.groupTeamMappings.length > 0 ? "Group claim *" : "Group claim"}
						description="ID token claim containing group values."
					>
						<input
							value={settings.groupClaimName}
							required={settings.groupTeamMappings.length > 0}
							onChange={(event) => update("groupClaimName", event.currentTarget.value)}
						/>
					</SettingsField>
				)}
				{settings.provider !== "google" && (
					<div className={styles.groupMappings}>
						<strong className={styles.mappingLabel}>Team mappings</strong>
						<p>Match {settings.provider === "oidc" ? "claim values" : "Entra group IDs"} to teams at signup.</p>
						{settings.groupTeamMappings.map((mapping, index) => (
							<div className={styles.mappingRow} key={index}>
								<input
									aria-label={`${settings.provider === "oidc" ? "Claim value" : "Group ID"} ${index + 1}`}
									placeholder={settings.provider === "oidc" ? "Claim value *" : "Group ID *"}
									required
									value={mapping.groupId}
									onChange={(event) =>
										update(
											"groupTeamMappings",
											settings.groupTeamMappings.map((entry, position) =>
												position === index ? { ...entry, groupId: event.currentTarget.value } : entry,
											),
										)
									}
								/>
								<select
									aria-label={`Team for mapping ${index + 1}`}
									required
									value={mapping.teamId}
									onChange={(event) =>
										update(
											"groupTeamMappings",
											settings.groupTeamMappings.map((entry, position) =>
												position === index ? { ...entry, teamId: event.currentTarget.value } : entry,
											),
										)
									}
								>
									<option value="">Select team *</option>
									{teams.map((team) => (
										<option value={team.id} key={team.id}>
											{team.displayName}
										</option>
									))}
								</select>
								<button
									type="button"
									className="button-secondary"
									aria-label={`Remove mapping ${index + 1}`}
									onClick={() =>
										update(
											"groupTeamMappings",
											settings.groupTeamMappings.filter((_, position) => position !== index),
										)
									}
								>
									<Trash2Icon size={16} />
								</button>
							</div>
						))}
						<button
							type="button"
							className="button-secondary"
							onClick={() => update("groupTeamMappings", [...settings.groupTeamMappings, { groupId: "", teamId: "" }])}
						>
							<PlusIcon size={16} /> Add mapping
						</button>
					</div>
				)}
			</section>
		)}
		{settings.enabled && (
			<div className={styles.configuration}>
				<h2 className={styles.sectionHeading}>Provider configuration</h2>
				{settings.provider === "oidc" && (
					<SettingsField
						label="Sign-in button label *"
						description='Appears as "Continue with [label]" on the sign-in page.'
						name="displayName"
					>
						<input
							name="displayName"
							value={settings.displayName}
							required
							onChange={(event) => update("displayName", event.currentTarget.value)}
						/>
					</SettingsField>
				)}
				<SettingsField
					label={settings.provider === "microsoft" ? "Application (client) ID *" : "Client ID *"}
					name="clientId"
				>
					<input
						name="clientId"
						value={settings.clientId}
						required
						onChange={(event) => update("clientId", event.currentTarget.value)}
					/>
				</SettingsField>
				<SettingsField
					label={secretConfigured ? "Client secret" : "Client secret *"}
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
						required={!secretConfigured}
						onChange={(event) => setClientSecret(event.currentTarget.value)}
					/>
				</SettingsField>
				{settings.provider === "oidc" && (
					<SettingsField
						label="Issuer URL *"
						description="The base URL used to discover your provider's OpenID Connect configuration."
						name="issuerUrl"
					>
						<input
							type="url"
							name="issuerUrl"
							value={settings.issuerUrl ?? ""}
							required
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
				{settings.provider === "oidc" && (
					<SettingsField
						label="Additional scopes"
						description="Space-separated scopes to request in addition to openid, profile, and email."
					>
						<input
							value={settings.additionalScopes}
							onChange={(event) => update("additionalScopes", event.currentTarget.value)}
						/>
					</SettingsField>
				)}
				{settings.provider === "microsoft" && (
					<SettingsField
						label="Directory (tenant) ID *"
						description="The directory ID for the Microsoft Entra tenant that can sign in."
						name="tenantId"
					>
						<input
							name="tenantId"
							value={settings.tenantId ?? ""}
							required
							onChange={(event) => update("tenantId", event.currentTarget.value || null)}
						/>
					</SettingsField>
				)}
				<SettingsField
					label="Callback URL"
					description={
						settings.provider === "microsoft"
							? "Add this URL as a Web redirect URI in Microsoft Entra ID."
							: "Add this URL to the provider's allowed redirect URLs."
					}
				>
					<CopyableValue value={settings.callbackUrl} label="Callback URL" />
				</SettingsField>
			</div>
		)}
	</>
);

export const AuthenticationSettingsPage = () => {
	const { teams } = useTeams();
	const [form, setForm] = useState<{ settings?: ExternalAuthSettings; clientSecret: string }>({ clientSecret: "" });
	const { settings, clientSecret } = form;
	const [savedSettings, setSavedSettings] = useState<ExternalAuthSettings>();
	const [error, setError] = useState<string>();
	const providerDrafts = useRef<
		Partial<Record<ExternalAuthProvider, { settings: ExternalAuthSettings; clientSecret: string }>>
	>({});

	useEffect(() => {
		api["/api/dashboard/admin/auth"]
			.get()
			.json()
			.then((settings) => {
				const next = withTeamDefaults(settings);
				setForm({ settings: next, clientSecret: "" });
				setSavedSettings(next);
				providerDrafts.current[next.provider] = { settings: next, clientSecret: "" };
			})
			.catch((error) => {
				setError(errorMessage(error));
				createToast(errorMessage(error), "error");
			});
	}, []);

	if (error && !settings) return null;
	if (!settings) return <LoadingSpinner />;

	const update = <K extends keyof ExternalAuthSettings>(key: K, value: ExternalAuthSettings[K]) =>
		setForm({ ...form, settings: { ...settings, [key]: value } });
	const selectProvider = (provider: ExternalAuthProvider | "internal") => {
		providerDrafts.current[settings.provider] = { settings, clientSecret };
		if (provider === "internal") {
			setForm({ ...form, settings: { ...settings, enabled: false } });
			return;
		}
		if (provider === settings.provider) {
			setForm({ ...form, settings: { ...settings, enabled: true } });
			return;
		}

		const draft = providerDrafts.current[provider];
		if (draft) {
			setForm({ settings: { ...draft.settings, enabled: true }, clientSecret: draft.clientSecret });
			return;
		}

		setForm({
			clientSecret: "",
			settings: {
				...settings,
				enabled: true,
				provider,
				displayName: providers.find((item) => item.value === provider)?.label ?? settings.displayName,
				clientId: "",
				issuerUrl: null,
				allowedDomain: null,
				tenantId: null,
				groupTeamMappings: [],
				additionalScopes: "",
				groupClaimName: "",
			},
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
			defaultTeamId: settings.defaultTeamId,
			groupTeamMappings: settings.provider === "google" ? [] : settings.groupTeamMappings,
			additionalScopes: settings.provider === "oidc" ? settings.additionalScopes : "",
			groupClaimName: settings.provider === "oidc" ? settings.groupClaimName : "",
		};

		api["/api/dashboard/admin/auth"]
			.put({ json: request })
			.json()
			.then((next) => {
				if (typeof next === "string") throw new Error(next);
				const normalized = withTeamDefaults(next);
				setForm({ settings: normalized, clientSecret: "" });
				setSavedSettings(normalized);
				providerDrafts.current[normalized.provider] = { settings: normalized, clientSecret: "" };
				createToast("Authentication settings updated", "success");
			})
			.catch((error) => createToast(errorMessage(error), "error"));
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
					setClientSecret={(clientSecret) => setForm({ ...form, clientSecret })}
					teams={teams}
				/>
			</SettingsForm>
		</div>
	);
};
