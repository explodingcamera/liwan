import dialogStyles from "./dialogs.module.css";
import styles from "./settings.module.css";

import { type FormEvent, useEffect, useMemo, useState } from "react";
import { CalendarDaysIcon, ChevronDownIcon, PlusIcon, SettingsIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { CopyableValue } from "@/components/ui/snippet";
import { type Column, Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath, basePath } from "@/config";
import type { ApiKey, ApiKeyExpiration } from "@/constants";
import { invalidateApiKeys, useApiKeys, useEntities, useProjects } from "@/hooks/api";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsHeader } from "./form";
import { type Tag, Tags } from "./tags";

type Permission = ApiKey["permissions"][number];
type Expiration = ApiKeyExpiration;
type AccessScope = "all" | "selected" | "none";
const permissions: Permission[] = ["events:batch"];
const defaultForm = {
	displayName: "",
	selectedEntities: [] as Tag[],
	selectedProjects: [] as Tag[],
	entityAccess: "none" as AccessScope,
	projectAccess: "none" as AccessScope,
	selectedPermissions: ["events:batch"] as Permission[],
	expiration: "30_days" as Expiration,
};

const keyForm = (key: ApiKey): typeof defaultForm => ({
	...defaultForm,
	displayName: key.displayName,
	selectedEntities: (key.entities === "all" ? [] : key.entities).map((id) => ({ value: id, label: id })),
	selectedProjects: (key.projects === "all" ? [] : key.projects).map((id) => ({ value: id, label: id })),
	entityAccess: key.entities === "all" ? "all" : key.entities.length ? "selected" : "none",
	projectAccess: key.projects === "all" ? "all" : key.projects.length ? "selected" : "none",
	selectedPermissions: key.permissions,
});
const expirationOptions: { value: Expiration; label: string }[] = [
	{ value: "never", label: "Never" },
	{ value: "7_days", label: "7 days" },
	{ value: "30_days", label: "30 days" },
	{ value: "60_days", label: "60 days" },
	{ value: "90_days", label: "90 days" },
];

export const ApiKeys = () => {
	const { entities } = useEntities();
	const { projects } = useProjects();
	const { keys, isLoading: loading } = useApiKeys();
	const [createOpen, setCreateOpen] = useState(false);
	const [form, setForm] = useState(defaultForm);
	const {
		displayName,
		selectedEntities,
		selectedProjects,
		entityAccess,
		projectAccess,
		selectedPermissions,
		expiration,
	} = form;
	const [plaintext, setPlaintext] = useState<string>();
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	const { mutate: createKey, isPending: creating } = useMutation({
		mutationFn: api["/api/dashboard/api-keys"].post,
		onSuccess: async (result) => {
			const response = await result.json();
			if (typeof response === "string") throw new Error(response);
			setCreateOpen(false);
			setPlaintext(response.plaintext);
			setForm(defaultForm);
			return invalidateApiKeys();
		},
		onError: () => createToast("Failed to create API key", "error"),
	});

	const create = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!displayName.trim() || creating) return;
		createKey({
			json: {
				displayName,
				entities:
					entityAccess === "all"
						? "all"
						: entityAccess === "selected"
							? selectedEntities.map((entity) => entity.value)
							: [],
				projects:
					projectAccess === "all"
						? "all"
						: projectAccess === "selected"
							? selectedProjects.map((project) => project.value)
							: [],
				permissions: selectedPermissions,
				expiration,
			},
		});
	};

	const columns: Column<ApiKey>[] = [
		{
			id: "name",
			header: "Name",
			render: (key) => <a href={appPath(`/settings/api-keys/${key.id}`)}>{key.displayName}</a>,
			nowrap: true,
			full: true,
		},
		{
			id: "expires",
			header: "Expires",
			render: (key) => {
				if (!key.expiresAt) return "Never";
				const expiration = new Date(key.expiresAt);
				return (
					<span>
						{expiration <= new Date() ? (
							<span className={styles.apiKeyExpired}>Expired on {expiration.toLocaleString()}</span>
						) : (
							expiration.toLocaleString()
						)}
					</span>
				);
			},
			nowrap: true,
		},
		{
			id: "lastUsed",
			header: "Last used",
			render: (key) => (key.lastUsedAt ? new Date(key.lastUsedAt).toLocaleString() : "Never"),
			nowrap: true,
		},
		{
			id: "created",
			header: "Created",
			render: (key) => new Date(key.createdAt).toLocaleDateString(),
			nowrap: true,
		},
		{
			id: "edit",
			render: (key) => (
				<a
					href={appPath(`/settings/api-keys/${key.id}`)}
					className={styles.settingsLink}
					aria-label={`Open ${key.displayName} settings`}
					title={`Open ${key.displayName} settings`}
				>
					<SettingsIcon size={18} />
				</a>
			),
		},
	];

	return (
		<>
			<nav className="list-header">
				<h1>API Keys</h1>
				<button
					type="button"
					className={dialogStyles.new}
					aria-label="Create API key"
					title="Create API key"
					onClick={() => setCreateOpen(true)}
				>
					<PlusIcon size={24} strokeWidth={2.25} aria-hidden="true" />
				</button>
			</nav>
			<p className="description">
				Manage credentials for server-side applications. Choose the access and permissions each key needs.
			</p>
			<Table columns={columns} rows={keys} isLoading={loading} emptyMessage="No API keys have been created." />

			<Dialog
				open={createOpen}
				onOpenChange={setCreateOpen}
				className={styles.apiKeyDialog}
				autoOverflow
				title="Create API key"
			>
				<form onSubmit={create}>
					<SettingsField label="Key name">
						<input
							required
							type="text"
							maxLength={100}
							value={displayName}
							onChange={(event) => setForm({ ...form, displayName: event.currentTarget.value })}
							placeholder="Production server"
						/>
					</SettingsField>
					<div className={styles.apiKeyAccessSection}>
						<div className={styles.apiKeyAccessGrid}>
							<div className={styles.apiKeyAccessRow}>
								<SettingsField label="Entities" description="Direct access to entities." htmlFor="create-entity-access">
									<div className={styles.apiKeySelect}>
										<select
											id="create-entity-access"
											value={entityAccess}
											onChange={(event) => setForm({ ...form, entityAccess: event.currentTarget.value as AccessScope })}
										>
											<option value="none">No entities</option>
											<option value="selected">Selected entities</option>
											<option value="all">All entities</option>
										</select>
										<ChevronDownIcon size={16} aria-hidden="true" />
									</div>
								</SettingsField>
								{entityAccess === "selected" && (
									<Tags
										labelText="Choose entities"
										selected={selectedEntities}
										suggestions={entityTags}
										onAdd={(entity) => setForm({ ...form, selectedEntities: [...selectedEntities, entity] })}
										onDelete={(index) =>
											setForm({ ...form, selectedEntities: selectedEntities.filter((_, i) => i !== index) })
										}
									/>
								)}
							</div>
							<div className={styles.apiKeyAccessRow}>
								<SettingsField
									label="Projects"
									description="Follows project membership."
									htmlFor="create-project-access"
								>
									<div className={styles.apiKeySelect}>
										<select
											id="create-project-access"
											value={projectAccess}
											onChange={(event) =>
												setForm({ ...form, projectAccess: event.currentTarget.value as AccessScope })
											}
										>
											<option value="none">No projects</option>
											<option value="selected">Selected projects</option>
											<option value="all">All projects</option>
										</select>
										<ChevronDownIcon size={16} aria-hidden="true" />
									</div>
								</SettingsField>
								{projectAccess === "selected" && (
									<Tags
										labelText="Choose projects"
										selected={selectedProjects}
										suggestions={projectTags}
										onAdd={(project) => setForm({ ...form, selectedProjects: [...selectedProjects, project] })}
										onDelete={(index) =>
											setForm({ ...form, selectedProjects: selectedProjects.filter((_, i) => i !== index) })
										}
									/>
								)}
							</div>
						</div>
						<SettingsFieldset legend="Permissions">
							<div className={styles.apiKeyPermissionOptions}>
								{permissions.map((permission) => (
									<label key={permission} className={styles.apiKeyPermissionOption}>
										<input
											type="checkbox"
											checked={selectedPermissions.includes(permission)}
											onChange={(event) => {
												const checked = event.currentTarget.checked;
												setForm({
													...form,
													selectedPermissions: checked
														? [...selectedPermissions, permission]
														: selectedPermissions.filter((value) => value !== permission),
												});
											}}
										/>
										<span>
											{permission}
											<small>Send event batches.</small>
										</span>
									</label>
								))}
							</div>
						</SettingsFieldset>
					</div>
					<SettingsField label="Expiration" htmlFor="create-key-expiration">
						<div className={`${styles.apiKeySelect} ${styles.apiKeyExpirationSelect}`}>
							<CalendarDaysIcon size={16} aria-hidden="true" />
							<select
								id="create-key-expiration"
								value={expiration}
								onChange={(event) => setForm({ ...form, expiration: event.currentTarget.value as Expiration })}
							>
								{expirationOptions.map((option) => (
									<option key={option.value} value={option.value}>
										{option.label}
									</option>
								))}
							</select>
							<ChevronDownIcon size={16} aria-hidden="true" />
						</div>
					</SettingsField>
					<div className="action-row">
						<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
						<button type="submit" className="button-primary" disabled={!displayName.trim() || creating}>
							{creating ? "Creating…" : "Create API key"}
						</button>
					</div>
				</form>
			</Dialog>

			<Dialog
				open={plaintext !== undefined}
				onOpenChange={(open) => !open && setPlaintext(undefined)}
				title="API key created"
				description="Save this key now. For security, it will not be shown again."
			>
				<form>
					<CopyableValue value={plaintext ?? ""} label="API key" />
					<div className="action-row">
						<Dialog.Close className="button-primary">Continue</Dialog.Close>
					</div>
				</form>
			</Dialog>
		</>
	);
};

export const ApiKeySettingsPage = ({ keyId: keyIdProp }: { keyId: string }) => {
	const { keys, isLoading: loading, error } = useApiKeys();
	const [keyId, setKeyId] = useState<string>();
	const key = keys.find((key) => key.id === keyId);

	useEffect(() => {
		const path = window.location.pathname.slice(basePath.length).replace(/\/$/, "");
		setKeyId(path.startsWith("/settings/api-keys/") ? path.slice("/settings/api-keys/".length) : keyIdProp);
	}, [keyIdProp]);
	useEffect(() => {
		if (!loading && !error && keyId && !key) createToast("API key not found", "error");
	}, [loading, error, keyId, key]);

	if (loading || !keyId) return <LoadingSpinner />;
	return key ? <ApiKeyEditor key={key.id} initialKey={key} /> : null;
};

const ApiKeyEditor = ({ initialKey }: { initialKey: ApiKey }) => {
	const { entities } = useEntities();
	const { projects } = useProjects();
	const [key, setKey] = useState(initialKey);
	const [form, setForm] = useState(() => keyForm(initialKey));
	const {
		displayName,
		selectedEntities,
		selectedProjects,
		entityAccess,
		projectAccess,
		selectedPermissions,
		expiration,
	} = form;
	const [plaintext, setPlaintext] = useState<string>();
	const [regenerateOpen, setRegenerateOpen] = useState(false);
	const [deleteOpen, setDeleteOpen] = useState(false);
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	const { mutate: updateKey, isPending: saving } = useMutation({
		mutationFn: api["/api/dashboard/api-keys/{key_id}"].put,
		onSuccess: (_, request) => {
			setKey((key) => ({ ...key, ...request.json }));
			createToast("API key updated", "success");
			return invalidateApiKeys();
		},
		onError: () => {
			setForm(keyForm(key));
			createToast("Failed to update API key", "error");
		},
	});
	const { mutate: deleteKey } = useMutation({
		mutationFn: api["/api/dashboard/api-keys/{key_id}"].delete,
		onSuccess: () => {
			createToast("API key deleted", "success");
			window.location.href = appPath("/settings/api-keys");
		},
		onError: () => createToast("Failed to delete API key", "error"),
	});
	const { mutate: regenerateKey, isPending: regenerating } = useMutation({
		mutationFn: api["/api/dashboard/api-keys/{key_id}/regenerate"].post,
		onSuccess: async (result) => {
			const response = await result.json();
			if (typeof response === "string") throw new Error(response);
			setKey(response.key);
			setPlaintext(response.plaintext);
			setRegenerateOpen(false);
			return invalidateApiKeys();
		},
		onError: () => createToast("Failed to regenerate API key", "error"),
	});

	const expiresAt = key.expiresAt ? new Date(key.expiresAt) : null;

	const save = (next: ApiKey) => {
		if (saving) return;
		updateKey({
			params: { key_id: next.id },
			json: {
				displayName: next.displayName,
				entities: next.entities,
				projects: next.projects,
				permissions: next.permissions,
			},
		});
	};

	const regenerate = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (regenerating) return;
		regenerateKey({ params: { key_id: key.id }, json: { expiration } });
	};

	return (
		<>
			<SettingsForm>
				<SettingsHeader title={key.displayName} backHref={appPath("/settings/api-keys")} backLabel="Back to API Keys" />
				<fieldset disabled={saving || regenerating} className={`${styles.detailPanel} ${styles.apiKeyDetailPanel}`}>
					<SettingsField label="Name" description="Identifies this key in the dashboard." name="displayName">
						<input
							required
							maxLength={100}
							value={displayName}
							onChange={(event) => setForm({ ...form, displayName: event.currentTarget.value })}
							onBlur={(event) => {
								const name = event.currentTarget.value.trim();
								if (name && name !== key.displayName) save({ ...key, displayName: name });
							}}
						/>
					</SettingsField>
					<div className={styles.apiKeyAccessSection}>
						<div className={styles.apiKeyAccessGrid}>
							<div className={styles.apiKeyAccessRow}>
								<SettingsField label="Entities" description="Direct access to entities." htmlFor="edit-entity-access">
									<div className={styles.apiKeySelect}>
										<select
											id="edit-entity-access"
											value={entityAccess}
											onChange={(event) => {
												const next = event.currentTarget.value as AccessScope;
												setForm({ ...form, entityAccess: next });
												save({
													...key,
													entities:
														next === "all"
															? "all"
															: next === "selected"
																? selectedEntities.map((tag) => tag.value)
																: [],
												});
											}}
										>
											<option value="none">No entities</option>
											<option value="selected">Selected entities</option>
											<option value="all">All entities</option>
										</select>
										<ChevronDownIcon size={16} aria-hidden="true" />
									</div>
								</SettingsField>
								{entityAccess === "selected" && (
									<Tags
										labelText="Choose entities"
										selected={selectedEntities.map(
											(tag) => entityTags.find((option) => option.value === tag.value) ?? tag,
										)}
										suggestions={entityTags}
										onAdd={(entity) => {
											const next = [...selectedEntities, entity];
											setForm({ ...form, selectedEntities: next });
											save({ ...key, entities: next.map((tag) => tag.value) });
										}}
										onDelete={(index) => {
											const next = selectedEntities.filter((_, i) => i !== index);
											setForm({ ...form, selectedEntities: next });
											save({ ...key, entities: next.map((tag) => tag.value) });
										}}
									/>
								)}
							</div>
							<div className={styles.apiKeyAccessRow}>
								<SettingsField label="Projects" description="Follows project membership." htmlFor="edit-project-access">
									<div className={styles.apiKeySelect}>
										<select
											id="edit-project-access"
											value={projectAccess}
											onChange={(event) => {
												const next = event.currentTarget.value as AccessScope;
												setForm({ ...form, projectAccess: next });
												save({
													...key,
													projects:
														next === "all"
															? "all"
															: next === "selected"
																? selectedProjects.map((tag) => tag.value)
																: [],
												});
											}}
										>
											<option value="none">No projects</option>
											<option value="selected">Selected projects</option>
											<option value="all">All projects</option>
										</select>
										<ChevronDownIcon size={16} aria-hidden="true" />
									</div>
								</SettingsField>
								{projectAccess === "selected" && (
									<Tags
										labelText="Choose projects"
										selected={selectedProjects.map(
											(tag) => projectTags.find((option) => option.value === tag.value) ?? tag,
										)}
										suggestions={projectTags}
										onAdd={(project) => {
											const next = [...selectedProjects, project];
											setForm({ ...form, selectedProjects: next });
											save({ ...key, projects: next.map((tag) => tag.value) });
										}}
										onDelete={(index) => {
											const next = selectedProjects.filter((_, i) => i !== index);
											setForm({ ...form, selectedProjects: next });
											save({ ...key, projects: next.map((tag) => tag.value) });
										}}
									/>
								)}
							</div>
						</div>
						<SettingsFieldset legend="Permissions">
							<div className={styles.apiKeyPermissionOptions}>
								{permissions.map((permission) => (
									<label key={permission} className={styles.apiKeyPermissionOption}>
										<input
											type="checkbox"
											checked={selectedPermissions.includes(permission)}
											onChange={(event) => {
												const checked = event.currentTarget.checked;
												setForm({
													...form,
													selectedPermissions: checked
														? [...selectedPermissions, permission]
														: selectedPermissions.filter((value) => value !== permission),
												});
												save({
													...key,
													permissions: checked
														? [...selectedPermissions, permission]
														: selectedPermissions.filter((value) => value !== permission),
												});
											}}
										/>
										<span>
											{permission}
											<small>Send event batches.</small>
										</span>
									</label>
								))}
							</div>
						</SettingsFieldset>
					</div>
					<div className={styles.dangerZone}>
						<div>
							<strong>Regenerate API key</strong>
							<p>
								{expiresAt ? (
									expiresAt <= new Date() ? (
										<span className={styles.apiKeyExpired}>Expired on {expiresAt.toLocaleString()}</span>
									) : (
										`Expires on ${expiresAt.toLocaleString()}`
									)
								) : (
									"Never expires"
								)}
								. Regeneration replaces the secret immediately.
							</p>
						</div>
						<button type="button" className="button-secondary" onClick={() => setRegenerateOpen(true)}>
							Regenerate
						</button>
					</div>
					<div className={styles.dangerZone}>
						<div>
							<strong>Delete API key</strong>
							<p>This key will immediately stop working.</p>
						</div>
						<button
							type="button"
							className={`${styles.deleteButton} button-danger`}
							onClick={() => setDeleteOpen(true)}
						>
							Delete
						</button>
					</div>
				</fieldset>
			</SettingsForm>
			<Dialog
				open={deleteOpen}
				onOpenChange={setDeleteOpen}
				title="Delete API key"
				description={`Delete “${key.displayName}”? The key will immediately stop working.`}
			>
				<form
					onSubmit={(event) => {
						event.preventDefault();
						deleteKey({ params: { key_id: key.id } });
					}}
				>
					<div className="action-row">
						<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
						<button type="submit" className={`${styles.deleteButton} button-danger`}>
							Delete API key
						</button>
					</div>
				</form>
			</Dialog>
			<Dialog
				open={regenerateOpen}
				onOpenChange={setRegenerateOpen}
				title="Regenerate API key"
				description="The current secret will stop working immediately. Choose an expiration for the new secret."
			>
				<form onSubmit={regenerate}>
					<SettingsField label="Expiration" htmlFor="regenerate-key-expiration">
						<div className={`${styles.apiKeySelect} ${styles.apiKeyExpirationSelect}`}>
							<CalendarDaysIcon size={16} aria-hidden="true" />
							<select
								id="regenerate-key-expiration"
								value={expiration}
								onChange={(event) => setForm({ ...form, expiration: event.currentTarget.value as Expiration })}
							>
								{expirationOptions.map((option) => (
									<option key={option.value} value={option.value}>
										{option.label}
									</option>
								))}
							</select>
							<ChevronDownIcon size={16} aria-hidden="true" />
						</div>
					</SettingsField>
					<div className="action-row">
						<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
						<button type="submit" className="button-primary" disabled={regenerating}>
							Regenerate API key
						</button>
					</div>
				</form>
			</Dialog>
			<Dialog
				open={plaintext !== undefined}
				onOpenChange={(open) => !open && setPlaintext(undefined)}
				title="API key regenerated"
				description="Save this key now. The old secret no longer works and this one will not be shown again."
			>
				<form>
					<CopyableValue value={plaintext ?? ""} label="API key" />
					<div className="action-row">
						<Dialog.Close className="button-primary">Continue</Dialog.Close>
					</div>
				</form>
			</Dialog>
		</>
	);
};
