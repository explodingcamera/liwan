import { queryClient, req, useMutation, useQuery } from "./utils";
export { queryClient, req, useMutation, useQuery };

export type ReportGraph = number[];
export type ReportTable = Record<string, number>;
export type ReportStats = {
	totalViews: number;
	totalSessions: number;
	uniqueVisitors: number;
	avgViewsPerSession: number;
};
export type DateRange = { start: Date; end: Date };
export type Dimension = "path" | "fqdn" | "referrer" | "platform" | "browser" | "mobile" | "country" | "city";
export type FilterType = "equals" | "not_equals" | "contains" | "not_contains" | "is_null";
export type Metric = "views" | "sessions" | "uniqueVisitors" | "avgViewsPerSession";

export type Group = {
	displayName: string;
	entities: Record<string, string>;
};

export type StatsRequest = { range: DateRange };
export type GraphRequest = StatsRequest & {
	dimension: Dimension;
	metric: Metric;
};

export const fetchGroups = () => req<Record<string, Group>>("GET", "/api/dashboard/groups");

export const fetchGroupStats = (group: string, data: StatsRequest) =>
	req<ReportStats>("POST", `/api/dashboard/group/${group}/overview`, data);

export const fetchGroupGraph = (group: string, data: GraphRequest) =>
	req<ReportGraph>("POST", `/api/dashboard/group/${group}/graph`, data);

export const mutateLogin = (username: string, password: string) =>
	req("POST", "/api/dashboard/auth/login", { username, password });

export const mutateLogout = () => req("POST", "/api/dashboard/auth/logout");
