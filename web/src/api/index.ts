import { createClient, type NormalizeOAS, type OASModel } from "fets";
export { queryClient, useMutation, useQuery, getUsername } from "./utils";
import type dashboardspec from "./dashboard";
import { queryClient, useQuery } from "./utils";
import { useMemo } from "react";

export type DashboardSpec = NormalizeOAS<typeof dashboardspec>;
export type Metric = OASModel<DashboardSpec, "Metric">;
export type Dimension = OASModel<DashboardSpec, "Dimension">;
export type DimensionTableRow = OASModel<DashboardSpec, "DimensionTableRow">;
export type DateRange = OASModel<DashboardSpec, "DateRange">;
export type ProjectResponse = OASModel<DashboardSpec, "ProjectResponse">;
export type EntityResponse = OASModel<DashboardSpec, "EntityResponse">;
export type UserResponse = OASModel<DashboardSpec, "UserResponse">;
export type StatsResponse = OASModel<DashboardSpec, "StatsResponse">;

export const api = createClient<DashboardSpec>({
	globalParams: { credentials: "same-origin" },
	fetchFn(input, init) {
		return fetch(input, init).then((res) => {
			if (!res.ok) {
				return res
					.json()
					.catch((_) => Promise.reject({ status: res.status, message: res.statusText }))
					.then((body) => Promise.reject({ status: res.status, message: body?.message ?? res.statusText }));
			}
			return res;
		});
	},
});

export const metricNames: Record<Metric, string> = {
	views: "Total Views",
	sessions: "Total Sessions",
	unique_visitors: "Unique Visitors",
	avg_views_per_session: "Avg. Views Per Session",
};

export const dimensionNames: Record<Dimension, string> = {
	platform: "Platform",
	browser: "Browser",
	url: "URL",
	path: "Path",
	mobile: "Device Type",
	referrer: "Referrer",
	city: "City",
	country: "Country",
	fqdn: "Domain",
};

export const formatMetricVal = (metric: Metric, value: number) => {
	let res = value;
	if (metric === "avg_views_per_session") {
		res = value / 1000;
	}

	if (res >= 1000) {
		return `${(res / 1000).toFixed(1).replace(/\.0$/, "")}K`;
	}

	if (res >= 1000000) {
		return `${(res / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
	}

	return res.toFixed(1).replace(/\.0$/, "") || "0";
};

export const useMe = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["me"],
		staleTime: 60 * 1000, // 1 minute
		queryFn: () => api["/api/dashboard/auth/me"].get().json(),
	});
	return { role: data?.role, username: data?.username, isLoading };
};

export const useProjects = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["projects"],
		queryFn: () => api["/api/dashboard/projects"].get().json(),
	});
	return { projects: data?.projects ?? [], isLoading, error };
};

export const useEntities = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["entities"],
		queryFn: () => api["/api/dashboard/entities"].get().json(),
	});

	return { entities: data?.entities ?? [], isLoading, error };
};

export const useUsers = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["users"],
		queryFn: () => api["/api/dashboard/users"].get().json(),
	});
	return { users: data?.users ?? [], isLoading, error };
};

export const useDimension = ({
	project,
	dimension,
	metric,
	range,
}: {
	project: ProjectResponse;
	dimension: Dimension;
	metric: Metric;
	range: DateRange;
}): {
	data: DimensionTableRow[] | undefined;
	biggest: number;
	order: string[] | undefined;
} => {
	const { data } = useQuery({
		placeholderData: (prev) => prev,
		queryKey: ["dimension", project.id, dimension, metric, range],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/dimension"]
				.post({
					params: { project_id: project.id },
					json: {
						dimension,
						metric,
						range,
					},
				})
				.json(),
	});

	const biggest = useMemo(() => data?.data?.reduce((acc, d) => Math.max(acc, d.value), 0) ?? 0, [data]);
	const order = useMemo(() => data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue), [data]);

	return { data: data?.data, biggest, order };
};

export const invalidateProjects = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
export const invalidateEntities = () => queryClient.invalidateQueries({ queryKey: ["entities"] });
export const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["users"] });
