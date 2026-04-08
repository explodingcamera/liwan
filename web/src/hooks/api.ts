import { useEffect, useMemo, useState } from "react";

import { toDataPoints } from "../components/graph";
import type { Dimension, DimensionFilter, DimensionTableRow, Metric, ProjectResponse } from "../api/types";

import { api } from "../api/client";
import { queryClient, useQuery } from "../api/query";
import type { DateRange } from "../api/ranges";

const getStatusCode = (error: unknown) => (error as { status?: number } | undefined)?.status;

// If theres an auth error on any api call, but we somehow still have the username cookie
// reload the page to clear the cookie and reset the state
const useReloadOnUnauthorized = (error: unknown) => {
	useEffect(() => {
		if (!error) return;
		if (getStatusCode(error) === 401) {
			cookieStore.get("liwan-username").then((cookie) => {
				if (cookie) {
					// this forces a logout / makes sure we never have a username cookie without a valid session cookie
					return cookieStore.delete("liwan-username").then(() => {
						window.location.reload();
					});
				}
			});
		}
	}, [error]);
};

export const useMe = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["me"],
		staleTime: 30_000,
		refetchOnMount: false,
		retry: false,
		queryFn: () => api["/api/dashboard/auth/me"].get().json(),
	});

	const [mounted, setMounted] = useState(false);
	useEffect(() => {
		setMounted(true);
	}, []);

	const authError = getStatusCode(error) === 401;
	useReloadOnUnauthorized(error);
	return { role: data?.role, username: data?.username, isLoading: isLoading || !mounted, authError };
};

export const useConfig = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["config"],
		refetchOnMount: false,
		queryFn: () => api["/api/dashboard/config"].get().json(),
	});
	return { config: data, isLoading };
};

export const useProjects = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["projects"],
		queryFn: () => api["/api/dashboard/projects"].get().json(),
	});
	return { projects: data?.projects ?? [], isLoading, error };
};

export const useProject = (projectId?: string) => {
	const { data, isLoading, error } = useQuery({
		enabled: projectId !== undefined,
		queryKey: ["project", projectId],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}"].get({ params: { project_id: projectId as string } }).json(),
	});
	return { project: data, isLoading, error, notFound: getStatusCode(error) === 404 };
};

export const useEntities = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["entities"],
		refetchOnMount: true,
		retry: false,
		queryFn: () => api["/api/dashboard/entities"].get().json(),
	});

	const authError = getStatusCode(error) === 401;
	useReloadOnUnauthorized(error);

	return { entities: data?.entities ?? [], isLoading, error, authError };
};

export const useUsers = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["users"],
		refetchOnMount: true,
		retry: false,
		queryFn: () => api["/api/dashboard/users"].get().json(),
	});

	const authError = getStatusCode(error) === 401;
	useReloadOnUnauthorized(error);

	return { users: data?.users ?? [], isLoading, error, authError };
};

export const useDimension = ({
	project,
	dimension,
	metric,
	range,
	filters,
}: {
	project: ProjectResponse;
	dimension: Dimension;
	metric: Metric;
	filters: DimensionFilter[];
	range: DateRange;
}): {
	data: DimensionTableRow[] | undefined;
	biggest: number;
	order: string[] | undefined;
	isLoading: boolean;
	error: unknown;
} => {
	const { data, isLoading, error } = useQuery({
		placeholderData: (prev) => prev,
		queryKey: ["dimension", project.id, dimension, metric, range.cacheKey(), filters],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/dimension"]
				.post({
					params: { project_id: project.id },
					json: {
						dimension,
						filters,
						metric,
						range: range.toAPI(),
					},
				})
				.json()
				.then((req) => {
					if (typeof req === "string") {
						console.error("Error fetching graph data:", req);
						return Promise.reject(new Error(req));
					}
					return req;
				}),
	});

	return useMemo(() => {
		const biggest = data?.data?.reduce((acc, d) => Math.max(acc, d.value), 0) ?? 0;
		const order = data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue);
		return { data: data?.data, biggest, order, isLoading, error };
	}, [data, isLoading, error]);
};
export const useProjectGraph = ({
	projectId,
	metric,
	range,
	filters = [],
}: {
	projectId?: string;
	metric: Metric;
	range: DateRange;
	filters?: DimensionFilter[];
}) => {
	let refetchInterval: number | undefined;
	let staleTime = 1000 * 60 * 10;
	if (range.endsToday()) {
		refetchInterval = 1000 * 60;
		staleTime = 0;
	}
	const dataPoints = range.getGraphDataPoints();
	const queryKey = ["project_graph", projectId, range.cacheKey(), metric, filters, dataPoints];

	const {
		data: graph,
		isError,
		isLoading,
		isFetching,
		isPlaceholderData,
	} = useQuery({
		refetchInterval,
		staleTime,
		enabled: projectId !== undefined,
		queryKey,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/graph"]
				.post({
					json: { range: range.toAPI(), metric, dataPoints, filters },
					params: { project_id: projectId ?? "" },
				})
				.json()
				.then((req) => {
					if (typeof req === "string") {
						console.error("Error fetching graph data:", req);
						return Promise.reject(new Error(req));
					}
					return toDataPoints(req.data, range);
				}),
		placeholderData: (prev) => prev,
	});

	const isUpdating = isFetching && isPlaceholderData;

	return {
		graph,
		isLoading,
		isError,
		isUpdating,
	};
};

export const useProjectStats = ({
	projectId,
	metric,
	range,
	filters = [],
}: {
	projectId?: string;
	metric: Metric;
	range: DateRange;
	filters?: DimensionFilter[];
}) => {
	const {
		data: stats,
		isError,
		isLoading,
	} = useQuery({
		queryKey: ["project_stats", projectId, range.cacheKey(), metric, filters],

		enabled: projectId !== undefined,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/stats"]
				.post({ json: { range: range.toAPI(), filters }, params: { project_id: projectId ?? "" } })
				.json()
				.then((req) => {
					if (typeof req === "string") {
						console.error("Error fetching graph data:", req);
						return Promise.reject(new Error(req));
					}
					return req;
				}),
		placeholderData: (prev) => prev,
	});

	return {
		stats,
		isLoading,
		isError,
	};
};

export const invalidateProjects = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
export const invalidateEntities = () => queryClient.invalidateQueries({ queryKey: ["entities"] });
export const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["users"] });
