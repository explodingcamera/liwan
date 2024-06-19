import { queryClient, req, useMutation, useQuery } from "./utils";
export { queryClient, req, useMutation, useQuery };

export type Group = {
	id: string;
	displayName: string;
	entities: Record<string, string>;
};

export const fetchGroups = () => req<Group[]>("GET", "/api/dashboard/groups");

export const mutateLogin = (username: string, password: string) =>
	req("POST", "/api/dashboard/auth/login", { username, password });
export const mutateLogout = () => req("POST", "/api/dashboard/auth/logout");
