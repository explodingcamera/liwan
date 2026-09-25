import dialogStyles from "./dialogs.module.css";
import styles from "./settings.module.css";

import { type FormEvent, type SubmitEvent, useEffect, useMemo, useState } from "react";
import { PlusIcon, SettingsIcon } from "lucide-react";

import { api } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { CopyableValue } from "@/components/ui/snippet";
import { type Column, Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { useEntities } from "@/hooks/api";
import { SettingsField, SettingsFieldset, SettingsForm, SettingsHeader } from "./form";
import { type Tag, Tags } from "./tags";

type ApiKey = {
	id: string;
	displayName: string;
	entities: string[];
	permissions: "events:batch"[];
	createdAt: string;
	lastUsedAt?: string | null;
	revokedAt?: string | null;
};

export const ApiKeys = () => {
	const { entities } = useEntities();
	const [keys, setKeys] = useState<ApiKey[]>([]);
	const [loading, setLoading] = useState(true);
	const [createOpen, setCreateOpen] = useState(false);
	const [displayName, setDisplayName] = useState("");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [plaintext, setPlaintext] = useState<string>();
	const [creating, setCreating] = useState(false);
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
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
					entities: selectedEntities.map((entity) => entity.value),
					permissions: ["events:batch"],
				},
			})
			.json()
			.then((response) => {
				if (typeof response === "string") throw new Error(response);
				setCreateOpen(false);
				setPlaintext(response.plaintext);
				setDisplayName("");
				setSelectedEntities([]);
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
			id: "status",
			header: "Status",
			render: (key) => (
				<span className={key.revokedAt ? styles.apiKeyStatusRevoked : styles.apiKeyStatusActive}>
					{key.revokedAt ? "Revoked" : "Active"}
				</span>
			),
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
				Manage credentials for server-side applications that send events. Restrict each key to the entities the
				application needs.
			</p>
			<Table columns={columns} rows={keys} isLoading={loading} emptyMessage="No API keys have been created." />

			<Dialog
				open={createOpen}
				onOpenChange={setCreateOpen}
				title="Create API key"
				description="Choose which entities this application can send events for."
			>
				<form onSubmit={create}>
					<label>
						Key name
						<small>Use a name that identifies the application.</small>
						<input
							required
							type="text"
							maxLength={100}
							value={displayName}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
							placeholder="Production server"
						/>
					</label>
					<Tags
						labelText="Entity access"
						labelDescription="The key cannot send events until at least one entity is selected."
						selected={selectedEntities}
						suggestions={entityTags}
						onAdd={(entity) => setSelectedEntities((entities) => [...entities, entity])}
						onDelete={(index) => setSelectedEntities((entities) => entities.filter((_, i) => i !== index))}
						noOptionsText="No more entities"
					/>
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
	const [keyId, setKeyId] = useState<string>();
	const [key, setKey] = useState<ApiKey>();
	const [displayName, setDisplayName] = useState("");
	const [selectedEntities, setSelectedEntities] = useState<Tag[]>([]);
	const [loading, setLoading] = useState(true);
	const [saving, setSaving] = useState(false);
	const [revokeOpen, setRevokeOpen] = useState(false);
	const entityTags = useMemo(
		() => entities.map((entity) => ({ value: entity.id, label: entity.displayName })),
		[entities],
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
			.then((response) => setKey(response.keys.find((key) => key.id === keyId)))
			.catch(() => createToast("Failed to load API key", "error"))
			.finally(() => setLoading(false));
	}, [keyId]);

	useEffect(() => {
		if (!key) return;
		setDisplayName(key.displayName);
		setSelectedEntities(
			key.entities.map((id) => {
				const entity = entities.find((entity) => entity.id === id);
				return { value: id, label: entity?.displayName ?? id };
			}),
		);
	}, [key, entities]);

	if (loading) return <LoadingSpinner />;
	if (!key) return <p>API key not found.</p>;

	const save = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (saving || !displayName.trim()) return;
		setSaving(true);
		api["/api/dashboard/api-keys/{key_id}"]
			.put({
				params: { key_id: key.id },
				json: {
					displayName,
					entities: selectedEntities.map((entity) => entity.value),
					permissions: key.permissions,
				},
			})
			.then(() => {
				setKey({ ...key, displayName: displayName.trim(), entities: selectedEntities.map((entity) => entity.value) });
				createToast("API key updated", "success");
			})
			.catch(() => createToast("Failed to update API key", "error"))
			.finally(() => setSaving(false));
	};

	const revoke = () => {
		api["/api/dashboard/api-keys/{key_id}"]
			.delete({ params: { key_id: key.id } })
			.then(() => {
				createToast("API key revoked", "success");
				window.location.href = "/settings/api-keys";
			})
			.catch(() => createToast("Failed to revoke API key", "error"));
	};

	return (
		<>
			<SettingsForm id="api-key-settings" onSubmit={save}>
				<SettingsHeader
					title={
						<span className={styles.apiKeyDetailTitle}>
							{key.displayName}
							{key.revokedAt && <span className={styles.apiKeyStatusRevoked}>Revoked</span>}
						</span>
					}
					backHref="/settings/api-keys"
					backLabel="Back to API Keys"
					saveForm={key.revokedAt ? undefined : "api-key-settings"}
				/>
				<div className={`${styles.detailPanel} ${styles.userDetailPanel}`}>
					<SettingsField label="Name" description="Identifies this key in the dashboard." name="displayName">
						<input
							required
							maxLength={100}
							value={displayName}
							disabled={Boolean(key.revokedAt)}
							onChange={(event) => setDisplayName(event.currentTarget.value)}
						/>
					</SettingsField>
					<Tags
						labelText="Entity access"
						labelDescription="Choose which entities this key can send events for."
						selected={selectedEntities}
						suggestions={entityTags}
						disabled={Boolean(key.revokedAt) || saving}
						onAdd={(entity) => setSelectedEntities((entities) => [...entities, entity])}
						onDelete={(index) => setSelectedEntities((entities) => entities.filter((_, i) => i !== index))}
						noOptionsText="No more entities"
					/>
					<div className={styles.apiKeyPermissions}>
						<SettingsFieldset
							legend="Permissions"
							description="Permissions cannot be changed after the key is created."
						>
							<label className={styles.apiKeyPermissionOption}>
								<input type="checkbox" checked={key.permissions.includes("events:batch")} disabled />
								<span>
									<strong>events:batch</strong>
									<small>Send events to assigned entities.</small>
								</span>
							</label>
						</SettingsFieldset>
					</div>
					{!key.revokedAt && (
						<div className={styles.dangerZone}>
							<div>
								<strong>Revoke API key</strong>
								<p>Applications using this key will immediately stop being able to send events.</p>
							</div>
							<button
								type="button"
								className={`${styles.deleteButton} button-danger`}
								onClick={() => setRevokeOpen(true)}
							>
								Revoke
							</button>
						</div>
					)}
				</div>
			</SettingsForm>
			<Dialog
				open={revokeOpen}
				onOpenChange={setRevokeOpen}
				title="Revoke API key"
				description={`Revoke “${key.displayName}”? Applications using it will immediately stop being able to send events.`}
			>
				<form
					onSubmit={(event) => {
						event.preventDefault();
						revoke();
					}}
				>
					<div className="action-row">
						<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
						<button type="submit" className={`${styles.deleteButton} button-danger`}>
							Revoke API key
						</button>
					</div>
				</form>
			</Dialog>
		</>
	);
};
