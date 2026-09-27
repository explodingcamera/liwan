import dialogStyles from "./dialogs.module.css";
import styles from "./settings.module.css";

import { type FormEvent, useEffect, useMemo, useState } from "react";
import { CalendarDaysIcon, ChevronDownIcon, PlusIcon, SettingsIcon } from "lucide-react";

import { api } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { CopyableValue } from "@/components/ui/snippet";
import { type Column, Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { useEntities, useProjects } from "@/hooks/api";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsHeader } from "./form";
import { type Tag, Tags } from "./tags";

type ApiKey = {
	id: string;
	displayName: string;
	entities: "all" | string[];
	projects: "all" | string[];
	permissions: Permission[];
	createdAt: string;
	lastUsedAt?: string | null;
	expiresAt?: string | null;
};

type Permission = "events:batch";
type Expiration = "never" | "7_days" | "30_days" | "60_days" | "90_days";
type AccessScope = "all" | "selected" | "none";
const permissions: Permission[] = ["events:batch"];
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
	const [keys, setKeys] = useState<ApiKey[]>([]);
	const [loading, setLoading] = useState(true);
	const [createOpen, setCreateOpen] = useState(false);
	const [displayName, setDisplayName] = useState("");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);
	const [entityAccess, setEntityAccess] = useState<AccessScope>("none");
	const [projectAccess, setProjectAccess] = useState<AccessScope>("none");
	const [selectedPermissions, setSelectedPermissions] = useState<Permission[]>(["events:batch"]);
	const [expiration, setExpiration] = useState<Expiration>("30_days");
	const [plaintext, setPlaintext] = useState<string>();
	const [creating, setCreating] = useState(false);
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	const load = () => {
		setLoading(true);
		api["/api/dashboard/api-keys"]
			.get()
			.json()
			.then((response) => setKeys(response.keys))
			.catch(() => createToast("Failed to load API keys", "error"))
			.finally(() => setLoading(false));
	};

	useEffect(load, []);

	const create = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!displayName.trim() || creating) return;
		setCreating(true);
		api["/api/dashboard/api-keys"]
			.post({
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
			})
			.json()
			.then((response) => {
				if (typeof response === "string") throw new Error(response);
				setCreateOpen(false);
				setPlaintext(response.plaintext);
				setDisplayName("");
				setSelectedEntities([]);
				setSelectedProjects([]);
				setEntityAccess("none");
				setProjectAccess("none");
				setSelectedPermissions(["events:batch"]);
				setExpiration("30_days");
				load();
			})
			.catch(() => createToast("Failed to create API key", "error"))
			.finally(() => setCreating(false));
	};

	const columns: Column<ApiKey>[] = [
		{
			id: "name",
			header: "Name",
			render: (key) => <strong>{key.displayName}</strong>,
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
					href={`/settings/api-keys/${key.id}`}
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
					<label>
						Key name
						<input
							required
							type="text"
							maxLength={100}
							value={displayName}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
							placeholder="Production server"
						/>
					</label>
					<div className={styles.apiKeyAccessSection}>
						<div className={styles.apiKeyAccessGrid}>
							<div className={styles.apiKeyAccessRow}>
								<label htmlFor="create-entity-access">Entities</label>
								<small className={styles.apiKeyAccessHelp}>Direct access to entities.</small>
								<div className={styles.apiKeySelect}>
									<select
										id="create-entity-access"
										value={entityAccess}
										onChange={(event) => setEntityAccess(event.currentTarget.value as typeof entityAccess)}
									>
										<option value="none">No entities</option>
										<option value="selected">Selected entities</option>
										<option value="all">All entities</option>
									</select>
									<ChevronDownIcon size={16} aria-hidden="true" />
								</div>
								{entityAccess === "selected" && (
									<Tags
										labelText="Choose entities"
										selected={selectedEntities}
										suggestions={entityTags}
										onAdd={(entity) => setSelectedEntities((entities) => [...entities, entity])}
										onDelete={(index) => setSelectedEntities((entities) => entities.filter((_, i) => i !== index))}
										noOptionsText="No more entities"
									/>
								)}
							</div>
							<div className={styles.apiKeyAccessRow}>
								<label htmlFor="create-project-access">Projects</label>
								<small className={styles.apiKeyAccessHelp}>Follows project membership.</small>
								<div className={styles.apiKeySelect}>
									<select
										id="create-project-access"
										value={projectAccess}
										onChange={(event) => setProjectAccess(event.currentTarget.value as typeof projectAccess)}
									>
										<option value="none">No projects</option>
										<option value="selected">Selected projects</option>
										<option value="all">All projects</option>
									</select>
									<ChevronDownIcon size={16} aria-hidden="true" />
								</div>
								{projectAccess === "selected" && (
									<Tags
										labelText="Choose projects"
										selected={selectedProjects}
										suggestions={projectTags}
										onAdd={(project) => setSelectedProjects((projects) => [...projects, project])}
										onDelete={(index) => setSelectedProjects((projects) => projects.filter((_, i) => i !== index))}
										noOptionsText="No more projects"
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
												setSelectedPermissions((current) =>
													checked ? [...current, permission] : current.filter((value) => value !== permission),
												);
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
					<label>
						Expiration
						<div className={`${styles.apiKeySelect} ${styles.apiKeyExpirationSelect}`}>
							<CalendarDaysIcon size={16} aria-hidden="true" />
							<select value={expiration} onChange={(event) => setExpiration(event.currentTarget.value as Expiration)}>
								{expirationOptions.map((option) => (
									<option key={option.value} value={option.value}>
										{option.label}
									</option>
								))}
							</select>
							<ChevronDownIcon size={16} aria-hidden="true" />
						</div>
					</label>
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
	const { entities } = useEntities();
	const { projects } = useProjects();
	const [keyId, setKeyId] = useState<string>();
	const [key, setKey] = useState<ApiKey>();
	const [displayName, setDisplayName] = useState("");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [selectedProjects, setSelectedProjects] = useState<Tag[]>([]);
	const [entityAccess, setEntityAccess] = useState<AccessScope>("none");
	const [projectAccess, setProjectAccess] = useState<AccessScope>("none");
	const [selectedPermissions, setSelectedPermissions] = useState<Permission[]>([]);
	const [expiration, setExpiration] = useState<Expiration>("30_days");
	const [plaintext, setPlaintext] = useState<string>();
	const [regenerateOpen, setRegenerateOpen] = useState(false);
	const [regenerating, setRegenerating] = useState(false);
	const [loading, setLoading] = useState(true);
	const [deleteOpen, setDeleteOpen] = useState(false);
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
	);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	useEffect(() => {
		const path = window.location.pathname.replace(/\/$/, "");
		setKeyId(path.startsWith("/settings/api-keys/") ? path.slice("/settings/api-keys/".length) : keyIdProp);
	}, [keyIdProp]);

	useEffect(() => {
		if (!keyId) return;
		api["/api/dashboard/api-keys"]
			.get()
			.json()
			.then((response) => {
				const loaded = response.keys.find((key) => key.id === keyId);
				setKey(loaded);
				if (!loaded) return;
				setDisplayName(loaded.displayName);
				setEntityAccess(loaded.entities === "all" ? "all" : loaded.entities.length ? "selected" : "none");
				setProjectAccess(loaded.projects === "all" ? "all" : loaded.projects.length ? "selected" : "none");
				setSelectedPermissions(loaded.permissions);
				setSelectedProjects((loaded.projects === "all" ? [] : loaded.projects).map((id) => ({ value: id, label: id })));
				setSelectedEntities((loaded.entities === "all" ? [] : loaded.entities).map((id) => ({ value: id, label: id })));
			})
			.catch(() => createToast("Failed to load API key", "error"))
			.finally(() => setLoading(false));
	}, [keyId]);

	if (loading) return <LoadingSpinner />;
	if (!key) return <p>API key not found.</p>;
	const expiresAt = key.expiresAt ? new Date(key.expiresAt) : null;

	const save = (next: ApiKey) => {
		setKey(next);
		api["/api/dashboard/api-keys/{key_id}"]
			.put({
				params: { key_id: next.id },
				json: {
					displayName: next.displayName,
					entities: next.entities,
					projects: next.projects,
					permissions: next.permissions,
				},
			})
			.then(() => createToast("API key updated", "success"))
			.catch(() => createToast("Failed to update API key", "error"));
	};

	const remove = () => {
		api["/api/dashboard/api-keys/{key_id}"]
			.delete({ params: { key_id: key.id } })
			.then(() => {
				createToast("API key deleted", "success");
				window.location.href = "/settings/api-keys";
			})
			.catch(() => createToast("Failed to delete API key", "error"));
	};

	const regenerate = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (regenerating) return;
		setRegenerating(true);
		api["/api/dashboard/api-keys/{key_id}/regenerate"]
			.post({ params: { key_id: key.id }, json: { expiration } })
			.json()
			.then((response) => {
				if (typeof response === "string") throw new Error(response);
				setKey((current) => current && { ...current, expiresAt: response.key.expiresAt, lastUsedAt: null });
				setPlaintext(response.plaintext);
				setRegenerateOpen(false);
			})
			.catch(() => createToast("Failed to regenerate API key", "error"))
			.finally(() => setRegenerating(false));
	};

	return (
		<>
			<SettingsForm>
				<SettingsHeader title={key.displayName} backHref="/settings/api-keys" backLabel="Back to API Keys" />
				<div className={`${styles.detailPanel} ${styles.userDetailPanel}`}>
					<SettingsField label="Name" description="Identifies this key in the dashboard." name="displayName">
						<input
							required
							maxLength={100}
							value={displayName}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
							onBlur={(event) => {
								const name = event.currentTarget.value.trim();
								if (name && name !== key.displayName) save({ ...key, displayName: name });
							}}
						/>
					</SettingsField>
					<div className={styles.apiKeyAccessSection}>
						<div className={styles.apiKeyAccessGrid}>
							<div className={styles.apiKeyAccessRow}>
								<label htmlFor="edit-entity-access">Entities</label>
								<small className={styles.apiKeyAccessHelp}>Direct access to entities.</small>
								<div className={styles.apiKeySelect}>
									<select
										id="edit-entity-access"
										value={entityAccess}
										onChange={(event) => {
											const next = event.currentTarget.value as AccessScope;
											setEntityAccess(next);
											save({
												...key,
												entities:
													next === "all" ? "all" : next === "selected" ? selectedEntities.map((tag) => tag.value) : [],
											});
										}}
									>
										<option value="none">No entities</option>
										<option value="selected">Selected entities</option>
										<option value="all">All entities</option>
									</select>
									<ChevronDownIcon size={16} aria-hidden="true" />
								</div>
								{entityAccess === "selected" && (
									<Tags
										labelText="Choose entities"
										selected={selectedEntities.map(
											(tag) => entityTags.find((option) => option.value === tag.value) ?? tag,
										)}
										suggestions={entityTags}
										onAdd={(entity) => {
											const next = [...selectedEntities, entity];
											setSelectedEntities(next);
											save({ ...key, entities: next.map((tag) => tag.value) });
										}}
										onDelete={(index) => {
											const next = selectedEntities.filter((_, i) => i !== index);
											setSelectedEntities(next);
											save({ ...key, entities: next.map((tag) => tag.value) });
										}}
										noOptionsText="No more entities"
									/>
								)}
							</div>
							<div className={styles.apiKeyAccessRow}>
								<label htmlFor="edit-project-access">Projects</label>
								<small className={styles.apiKeyAccessHelp}>Follows project membership.</small>
								<div className={styles.apiKeySelect}>
									<select
										id="edit-project-access"
										value={projectAccess}
										onChange={(event) => {
											const next = event.currentTarget.value as AccessScope;
											setProjectAccess(next);
											save({
												...key,
												projects:
													next === "all" ? "all" : next === "selected" ? selectedProjects.map((tag) => tag.value) : [],
											});
										}}
									>
										<option value="none">No projects</option>
										<option value="selected">Selected projects</option>
										<option value="all">All projects</option>
									</select>
									<ChevronDownIcon size={16} aria-hidden="true" />
								</div>
								{projectAccess === "selected" && (
									<Tags
										labelText="Choose projects"
										selected={selectedProjects.map(
											(tag) => projectTags.find((option) => option.value === tag.value) ?? tag,
										)}
										suggestions={projectTags}
										onAdd={(project) => {
											const next = [...selectedProjects, project];
											setSelectedProjects(next);
											save({ ...key, projects: next.map((tag) => tag.value) });
										}}
										onDelete={(index) => {
											const next = selectedProjects.filter((_, i) => i !== index);
											setSelectedProjects(next);
											save({ ...key, projects: next.map((tag) => tag.value) });
										}}
										noOptionsText="No more projects"
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
												setSelectedPermissions((current) =>
													checked ? [...current, permission] : current.filter((value) => value !== permission),
												);
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
				</div>
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
						remove();
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
					<label>
						Expiration
						<div className={`${styles.apiKeySelect} ${styles.apiKeyExpirationSelect}`}>
							<CalendarDaysIcon size={16} aria-hidden="true" />
							<select value={expiration} onChange={(event) => setExpiration(event.currentTarget.value as Expiration)}>
								{expirationOptions.map((option) => (
									<option key={option.value} value={option.value}>
										{option.label}
									</option>
								))}
							</select>
							<ChevronDownIcon size={16} aria-hidden="true" />
						</div>
					</label>
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
