import { useMemo } from "react";

import { toDataPoints } from "../components/graph";
import type { Dimension, DimensionFilter, DimensionTableRow, Metric, ProjectResponse } from "./types";

import { api } from ".";
import { queryClient, useQuery } from "./query";
import type { DateRange } from "./ranges";

export const useMe = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["me"],
		refetchOnMount: false,
		queryFn: () => api["/api/dashboard/auth/me"].get().json(),
	});
	return { role: data?.role, username: data?.username, isLoading };
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
	return { project: data, isLoading, error };
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
				.json(),
	});

	const biggest = useMemo(() => data?.data?.reduce((acc, d) => Math.max(acc, d.value), 0) ?? 0, [data]);
	const order = useMemo(() => data?.data?.sort((a, b) => b.value - a.value).map((d) => d.dimensionValue), [data]);

	return { data: data?.data, biggest, order, isLoading, error };
};

export const useProjectData = ({
	project,
	metric,
	range,
	filters = [],
}: {
	project?: ProjectResponse;
	metric: Metric;
	range: DateRange;
	filters?: DimensionFilter[];
}) => {
	let refetchInterval = undefined;
	let staleTime = 1000 * 60 * 10;
	if (range.endsToday()) {
		refetchInterval = 1000 * 60;
		staleTime = 0;
	}
	const dataPoints = range.getGraphDataPoints();

	const {
		data: stats,
		isError: isErrorStats,
		isLoading: isLoadingStats,
	} = useQuery({
		refetchInterval,
		staleTime,
		queryKey: ["project_stats", project?.id, range.cacheKey(), metric, filters],

		enabled: project !== undefined,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/stats"]
				.post({ json: { range: range.toAPI(), filters }, params: { project_id: project?.id ?? "" } })
				.json(),
		placeholderData: (prev) => prev,
	});

	const {
		data: graph,
		isError: isErrorGraph,
		isLoading: isLoadingGraph,
	} = useQuery({
		refetchInterval,
		staleTime,
		enabled: project !== undefined,
		queryKey: ["project_graph", project?.id, range.cacheKey(), metric, filters, dataPoints],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/graph"]
				.post({
					json: { range: range.toAPI(), metric, dataPoints, filters },
					params: { project_id: project?.id ?? "" },
				})
				.json(),
		placeholderData: (prev) => prev,
	});

	return {
		stats: {
			error: isErrorStats,
			loading: isLoadingStats,
			data: stats,
		},
		graph: {
			error: isErrorGraph,
			loading: isLoadingGraph,
			range,
			data: graph?.data ? toDataPoints(graph.data, range, metric) : [],
		},
		isLoading: isLoadingStats || isLoadingGraph,
		isError: isErrorStats || isErrorGraph,
	};
};

export type ProjectDataGraph = ReturnType<typeof useProjectData>["graph"];
export type ProjectDataStats = ReturnType<typeof useProjectData>["stats"];

export const invalidateProjects = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
export const invalidateEntities = () => queryClient.invalidateQueries({ queryKey: ["entities"] });
export const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["users"] });
