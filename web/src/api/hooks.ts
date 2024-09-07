import { useMemo } from "react";

import { toDataPoints } from "../components/graph";
import type { DateRange, Dimension, DimensionTableRow, Metric, ProjectResponse } from "./types";

import { api } from ".";
import { queryClient, useQuery } from "./query";
import { resolveRange, type RangeName } from "./ranges";

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
}: {
	project: ProjectResponse;
	dimension: Dimension;
	metric: Metric;
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

	return { data: data?.data, biggest, order, isLoading, error };
};

export const useProjectData = ({
	project,
	metric,
	rangeName = "last7Days",
}: {
	project?: ProjectResponse;
	metric: Metric;
	rangeName?: RangeName;
}) => {
	const { range, graphRange, dataPoints } = useMemo(() => resolveRange(rangeName), [rangeName]);

	let refetchInterval = undefined;
	let staleTime = 1000 * 60 * 10;
	if (rangeName === "today" || rangeName.startsWith("last")) {
		refetchInterval = 1000 * 60;
		staleTime = 0;
	}

	const {
		data: stats,
		isError: isErrorStats,
		isLoading: isLoadingStats,
	} = useQuery({
		refetchInterval,
		staleTime,
		queryKey: ["project_stats", project?.id, range],
		enabled: project !== undefined,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/stats"]
				.post({ json: { range }, params: { project_id: project?.id ?? "" } })
				.json(),
		placeholderData: (prev) => prev,
	});

	const json = { range, metric, dataPoints };
	const {
		data: graph,
		isError: isErrorGraph,
		isLoading: isLoadingGraph,
	} = useQuery({
		refetchInterval,
		staleTime,
		enabled: project !== undefined,
		queryKey: ["project_graph", project?.id, range, graphRange, metric, dataPoints],
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/graph"].post({ json, params: { project_id: project?.id ?? "" } }).json(),
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
			range: graphRange,
			data: graph?.data ? toDataPoints(graph.data, range, metric) : [],
		},
		isLoading: isLoadingStats || isLoadingGraph,
		isError: isErrorStats || isErrorGraph,
	};
};

export const invalidateProjects = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
export const invalidateEntities = () => queryClient.invalidateQueries({ queryKey: ["entities"] });
export const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["users"] });
