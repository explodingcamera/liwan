import dialogStyles from "./dialogs.module.css";
import styles from "./settings.module.css";

import { type FormEvent, useEffect, useMemo, useState } from "react";
import { CalendarDaysIcon, PlusIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { CopyableValue } from "@/components/ui/snippet";
import { type Column, Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath } from "@/config";
import type { ApiKey, ApiKeyExpiration } from "@/constants";
import { invalidateApiKeys, useApiKeys, useEntities, useProjects } from "@/hooks/api";
import { getSettingsPathId, SettingsField, SettingsFieldset, SettingsForm, SettingsHeader, SettingsLink } from "./form";
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

type KeyForm = typeof defaultForm;
type FieldProps = { id: string; form: KeyForm; onChange: (form: KeyForm) => void };

const accessValue = (scope: AccessScope, tags: Tag[]) => {
	if (scope === "all") return "all";
	if (scope === "selected") return tags.map((tag) => tag.value);
	return [];
};

const AccessFields = ({ id, form, onChange }: FieldProps) => {
	const { entities } = useEntities();
	const { projects } = useProjects();
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);
	return (
		<div className={styles.apiKeyAccessSection}>
			<div className={styles.apiKeyAccessGrid}>
				<div className={styles.apiKeyAccessRow}>
					<SettingsField label="Entities" description="Direct access to entities." htmlFor={`${id}-entities-access`}>
						<select
							id={`${id}-entities-access`}
							value={form.entityAccess}
							onChange={(event) => onChange({ ...form, entityAccess: event.currentTarget.value as AccessScope })}
						>
							<option value="none">No entities</option>
							<option value="selected">Selected entities</option>
							<option value="all">All entities</option>
						</select>
					</SettingsField>
					{form.entityAccess === "selected" && (
						<Tags
							labelText="Choose entities"
							selected={form.selectedEntities.map(
								(tag) => entityTags.find((option) => option.value === tag.value) ?? tag,
							)}
							suggestions={entityTags}
							onAdd={(tag) => onChange({ ...form, selectedEntities: [...form.selectedEntities, tag] })}
							onDelete={(index) =>
								onChange({ ...form, selectedEntities: form.selectedEntities.filter((_, i) => i !== index) })
							}
						/>
					)}
				</div>
				<div className={styles.apiKeyAccessRow}>
					<SettingsField label="Projects" description="Follows project membership." htmlFor={`${id}-projects-access`}>
						<select
							id={`${id}-projects-access`}
							value={form.projectAccess}
							onChange={(event) => onChange({ ...form, projectAccess: event.currentTarget.value as AccessScope })}
						>
							<option value="none">No projects</option>
							<option value="selected">Selected projects</option>
							<option value="all">All projects</option>
						</select>
					</SettingsField>
					{form.projectAccess === "selected" && (
						<Tags
							labelText="Choose projects"
							selected={form.selectedProjects.map(
								(tag) => projectTags.find((option) => option.value === tag.value) ?? tag,
							)}
							suggestions={projectTags}
							onAdd={(tag) => onChange({ ...form, selectedProjects: [...form.selectedProjects, tag] })}
							onDelete={(index) =>
								onChange({ ...form, selectedProjects: form.selectedProjects.filter((_, i) => i !== index) })
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
								checked={form.selectedPermissions.includes(permission)}
								onChange={(event) =>
									onChange({
										...form,
										selectedPermissions: event.currentTarget.checked
											? [...form.selectedPermissions, permission]
											: form.selectedPermissions.filter((value) => value !== permission),
									})
								}
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
	);
};

const ExpirationField = ({ id, form, onChange }: FieldProps) => (
	<SettingsField label="Expiration" htmlFor={id}>
		<div className={styles.apiKeyExpirationSelect}>
			<CalendarDaysIcon size={16} aria-hidden="true" />
			<select
				id={id}
				value={form.expiration}
				onChange={(event) => onChange({ ...form, expiration: event.currentTarget.value as Expiration })}
			>
				{expirationOptions.map((option) => (
					<option key={option.value} value={option.value}>
						{option.label}
					</option>
				))}
			</select>
		</div>
	</SettingsField>
);

export const ApiKeys = () => {
	const { keys, isLoading: loading } = useApiKeys();
	const [createOpen, setCreateOpen] = useState(false);
	const [form, setForm] = useState(defaultForm);
	const { displayName } = form;
	const [plaintext, setPlaintext] = useState<string>();

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
				entities: accessValue(form.entityAccess, form.selectedEntities),
				projects: accessValue(form.projectAccess, form.selectedProjects),
				permissions: form.selectedPermissions,
				expiration: form.expiration,
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
							<span className={styles.apiKeyExpired}>Expired on {expiration.toLocaleDateString()}</span>
						) : (
							expiration.toLocaleDateString()
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
				<SettingsLink href={appPath(`/settings/api-keys/${key.id}`)} label={`Open ${key.displayName} settings`} />
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
					<AccessFields id="create" form={form} onChange={setForm} />
					<ExpirationField id="create-key-expiration" form={form} onChange={setForm} />
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
		setKeyId(getSettingsPathId("/settings/api-keys/") || keyIdProp);
	}, [keyIdProp]);
	useEffect(() => {
		if (!loading && !error && keyId && !key) createToast("API key not found", "error");
	}, [loading, error, keyId, key]);

	if (loading || !keyId) return <LoadingSpinner />;
	return key ? <ApiKeyEditor key={key.id} initialKey={key} /> : null;
};

const ApiKeyEditor = ({ initialKey }: { initialKey: ApiKey }) => {
	const [key, setKey] = useState(initialKey);
	const [form, setForm] = useState(() => keyForm(initialKey));
	const [plaintext, setPlaintext] = useState<string>();
	const [regenerateOpen, setRegenerateOpen] = useState(false);
	const [deleteOpen, setDeleteOpen] = useState(false);

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
		regenerateKey({ params: { key_id: key.id }, json: { expiration: form.expiration } });
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
							value={form.displayName}
							onChange={(event) => setForm({ ...form, displayName: event.currentTarget.value })}
							onBlur={(event) => {
								const name = event.currentTarget.value.trim();
								if (name && name !== key.displayName) save({ ...key, displayName: name });
							}}
						/>
					</SettingsField>
					<AccessFields
						id="edit"
						form={form}
						onChange={(next) => {
							setForm(next);
							save({
								...key,
								entities: accessValue(next.entityAccess, next.selectedEntities),
								projects: accessValue(next.projectAccess, next.selectedProjects),
								permissions: next.selectedPermissions,
							});
						}}
					/>
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
						<button type="button" className="button-danger" onClick={() => setDeleteOpen(true)}>
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
						<button type="submit" className="button-danger">
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
					<ExpirationField id="regenerate-key-expiration" form={form} onChange={setForm} />
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
