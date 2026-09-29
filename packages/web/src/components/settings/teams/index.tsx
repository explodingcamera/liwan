import dialogStyles from "../dialogs.module.css";
import styles from "../settings.module.css";

import type { SubmitEvent } from "react";
import { useEffect, useMemo, useState } from "react";
import { navigate } from "astro:transitions/client";
import { ChevronDownIcon, PlusIcon, SettingsIcon } from "lucide-react";

import { api, useMutation } from "@/api";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import type { Column } from "@/components/ui/table";
import { Table } from "@/components/ui/table";
import { createToast } from "@/components/ui/toast";
import { appPath, basePath } from "@/config";
import { invalidateTeams, useProjects, useTeams, useUsers } from "@/hooks/api";
import { DeleteDialog } from "../dialogs";
import { SettingsField, SettingsForm, SettingsHeader } from "../form";
import type { Tag } from "../tags";
import { Tags } from "../tags";

export const CreateTeam = () => {
	const { mutate } = useMutation({
		mutationFn: api["/api/dashboard/teams"].post,
		onSuccess: async (result) => {
			const response = await result.json();
			if (typeof response === "string") throw new Error(response);
			invalidateTeams();
			createToast("Team created", "success");
			navigate(appPath(`/settings/teams/${response.id}`));
		},
		onError: (error) => createToast(error.message, "error"),
	});
	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		const data = new FormData(event.currentTarget);
		mutate({ json: { displayName: String(data.get("displayName")) } });
	};
	return (
		<Dialog
			title="Create a team"
			description="Add a team to manage project access."
			trigger={
				<button type="button" className={dialogStyles.new} aria-label="Create team" title="Create team">
					<PlusIcon size={24} />
				</button>
			}
		>
			<form onSubmit={handleSubmit}>
				<SettingsField label="Team name">
					<input name="displayName" required autoComplete="off" />
				</SettingsField>
				<div className="action-row">
					<Dialog.Close className="button-secondary">Cancel</Dialog.Close>
					<button type="submit" className="button-primary">
						Create team
					</button>
				</div>
			</form>
		</Dialog>
	);
};

export const TeamsTable = () => {
	const { teams, isLoading, error } = useTeams();
	if (error) return <p>Could not load teams.</p>;
	const columns: Column<(typeof teams)[number]>[] = [
		{
			id: "name",
			header: "Name",
			render: (team) => <a href={appPath(`/settings/teams/${team.id}`)}>{team.displayName}</a>,
			nowrap: true,
		},
		{ id: "users", header: "Members", render: (team) => String(team.users.length), full: true },
		{
			id: "edit",
			render: (team) => (
				<a
					className={styles.settingsLink}
					href={appPath(`/settings/teams/${team.id}`)}
					aria-label={`Open ${team.displayName} settings`}
				>
					<SettingsIcon size={18} />
				</a>
			),
		},
	];
	return <Table columns={columns} rows={teams} isLoading={isLoading} />;
};

export const TeamSettingsPage = ({ teamId }: { teamId: string }) => {
	const [id, setId] = useState(teamId);
	useEffect(() => {
		setId(window.location.pathname.slice(basePath.length).replace(/\/$/, "").split("/settings/teams/")[1] || teamId);
	}, [teamId]);
	return <TeamSettingsContent key={id} teamId={id} />;
};

const TeamSettingsContent = ({ teamId }: { teamId: string }) => {
	const { teams, isLoading, error } = useTeams();
	const { users, isLoading: usersLoading, authError: usersError } = useUsers();
	const { projects, isLoading: projectsLoading, error: projectsError } = useProjects();
	const team = teams.find((item) => item.id === teamId);
	const [name, setName] = useState("");
	const [members, setMembers] = useState<Tag[]>([]);
	const [access, setAccess] = useState<Tag[]>([]);
	const [projectScope, setProjectScope] = useState<"none" | "selected" | "all">("none");
	const [initialized, setInitialized] = useState(false);
	const userTags = useMemo(() => users.map((user) => ({ value: user.username, label: user.username })), [users]);
	const projectTags = useMemo(
		() => projects.map((project) => ({ value: project.id, label: project.displayName })),
		[projects],
	);

	useEffect(() => {
		if (!team || initialized) return;
		setName(team.displayName);
		setMembers(team.users.map((username) => ({ value: username, label: username })));
		setProjectScope(team.projects === "all" ? "all" : team.projects.length ? "selected" : "none");
		setAccess(
			(team.projects === "all" ? [] : team.projects).map((projectId) => ({
				value: projectId,
				label: projectId,
			})),
		);
		setInitialized(true);
	}, [team, initialized]);

	const { mutate: save } = useMutation({
		scope: { id: `team:${teamId}` },
		mutationFn: api["/api/dashboard/team/{team_id}"].put,
		onSuccess: () => {
			invalidateTeams();
			createToast("Team updated", "success");
		},
		onError: (failure) => createToast(failure.message, "error"),
	});
	const saveTeam = (nextName: string, nextMembers: Tag[], nextScope: typeof projectScope, nextAccess: Tag[]) => {
		if (!nextName.trim()) return;
		save({
			params: { team_id: teamId },
			json: {
				displayName: nextName,
				users: nextMembers.map((tag) => tag.value),
				projects: nextScope === "all" ? "all" : nextScope === "selected" ? nextAccess.map((tag) => tag.value) : [],
			},
		});
	};

	if (error || usersError || projectsError) return <p>Could not load team settings.</p>;
	if (isLoading || usersLoading || projectsLoading) return <LoadingSpinner />;
	if (!team) return <p>Team not found.</p>;
	if (!initialized) return <LoadingSpinner />;
	return (
		<SettingsForm>
			<SettingsHeader title={team.displayName} backHref={appPath("/settings/teams")} backLabel="Back to teams" />
			<div className={styles.detailPanel}>
				<SettingsField label="Team name">
					<input
						value={name}
						required
						onChange={(event) => setName(event.target.value)}
						onBlur={(event) => {
							if (event.currentTarget.value !== team.displayName) {
								saveTeam(event.currentTarget.value, members, projectScope, access);
							}
						}}
					/>
				</SettingsField>
				<Tags
					labelText="Members"
					labelDescription="Users in this team receive read access to its projects."
					selected={members}
					suggestions={userTags}
					onAdd={(tag) => {
						const next = [...members, tag];
						setMembers(next);
						saveTeam(name, next, projectScope, access);
					}}
					onDelete={(index) => {
						const next = members.filter((_, i) => i !== index);
						setMembers(next);
						saveTeam(name, next, projectScope, access);
					}}
				/>
				<SettingsField
					label="Project access"
					description="Choose which projects members can view."
					htmlFor="team-project-scope"
				>
					<div className={styles.apiKeySelect}>
						<select
							id="team-project-scope"
							value={projectScope}
							onChange={(event) => {
								const next = event.currentTarget.value as typeof projectScope;
								setProjectScope(next);
								saveTeam(name, members, next, access);
							}}
						>
							<option value="none">No projects</option>
							<option value="selected">Selected projects</option>
							<option value="all">All projects</option>
						</select>
						<ChevronDownIcon size={16} aria-hidden="true" />
					</div>
				</SettingsField>
				{projectScope === "selected" && (
					<Tags
						labelText="Choose projects"
						selected={access.map((tag) => ({
							...tag,
							label: projects.find((project) => project.id === tag.value)?.displayName ?? tag.label,
						}))}
						suggestions={projectTags}
						onAdd={(tag) => {
							const next = [...access, tag];
							setAccess(next);
							saveTeam(name, members, projectScope, next);
						}}
						onDelete={(index) => {
							const next = access.filter((_, i) => i !== index);
							setAccess(next);
							saveTeam(name, members, projectScope, next);
						}}
					/>
				)}
				<div className={styles.dangerZone}>
					<div>
						<strong>Delete team</strong>
						<p>Members will lose access granted by this team.</p>
					</div>
					<DeleteDialog
						id={team.id}
						displayName={team.displayName}
						type="team"
						onDeleted={() => navigate(appPath("/settings/teams"))}
						trigger={
							<button type="button" className="button-danger">
								Delete team
							</button>
						}
					/>
				</div>
			</div>
		</SettingsForm>
	);
};
