import { useEffect, useMemo, useState } from "react";

import { api } from "@/api/client";
import { queryClient, useQuery } from "@/api/query";
import type { DateRange } from "@/api/ranges";
import { createToast } from "@/components/ui/toast";
import type { Dimension, DimensionFilter, GraphInterval, Metric, ProjectResponse } from "@/constants";
import {
	type ComparisonRange,
	type ComparisonRow,
	getReportRanges,
	getStatsReportRanges,
	mergeComparisonRows,
} from "../components/dashboard/project/comparison";
import { toDataPoints } from "../components/dashboard/project/graph";

const numericCollator = new Intl.Collator(undefined, { numeric: true });

const getStatusCode = (error: unknown) => (error as { status?: number } | undefined)?.status;

export const useMe = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["me"],
		staleTime: 5 * 60_000,
		refetchOnMount: false,
		retry: false,
		queryFn: () => api["/api/dashboard/auth/me"].get().json(),
	});

	const [mounted, setMounted] = useState(false);
	useEffect(() => {
		setMounted(true);
	}, []);

	const authError = getStatusCode(error) === 401;
	return {
		role: data?.role,
		username: data?.username,
		isLoading: isLoading || !mounted,
		authError,
	};
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

	return {
		project: data,
		isLoading,
		error,
		notFound: getStatusCode(error) === 404,
	};
};

export const useCustomEvents = ({
	projectId,
	range,
	filters,
	enabled = true,
	comparisonRange,
}: {
	projectId?: string;
	range: DateRange;
	filters: DimensionFilter[];
	enabled?: boolean;
	comparisonRange?: ComparisonRange;
}) => {
	const { data, isFetching, isLoading, isPlaceholderData, error } = useQuery({
		queryKey: ["custom_events", projectId, range.cacheKey(), comparisonRange?.cacheKey(), filters],
		enabled: projectId !== undefined && enabled,
		refetchInterval: range.endsToday() || comparisonRange?.endsToday() ? 60_000 : undefined,
		placeholderData: (prev, previousQuery) => {
			if (!prev || previousQuery?.queryKey[1] !== projectId) return undefined;
			if (!comparisonRange && previousQuery?.queryKey[3] !== undefined) {
				return {
					...prev,
					rows: prev.rows.map((row) => ({ ...row, comparisonCompletions: undefined, comparisonUniques: undefined })),
				};
			}
			return prev;
		},
		queryFn: async () => {
			const result = await api["/api/dashboard/project/{project_id}/custom-events"]
				.post({
					params: { project_id: projectId ?? "" },
					json: { ranges: getReportRanges(range, comparisonRange), filters },
				})
				.json();
			if (typeof result === "string") throw new Error(result);
			const [current, previous] = result.reports;
			const previousRows = new Map(previous?.rows.map((row) => [row.name, row]));
			return {
				...current,
				truncated: current.truncated || previous?.truncated || false,
				rows: current.rows
					.map((row) => ({
						...row,
						comparisonCompletions: previousRows.get(row.name)?.completions,
						comparisonUniques: previousRows.get(row.name)?.uniques,
					}))
					.sort(
						(a, b) =>
							Number(a.completions == null) - Number(b.completions == null) ||
							(b.completions ?? 0) - (a.completions ?? 0) ||
							numericCollator.compare(a.name, b.name),
					),
			};
		},
	});
	return { data, isLoading: isLoading || (isFetching && isPlaceholderData), error };
};

export const useEntities = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["entities"],
		refetchOnMount: true,
		retry: false,
		queryFn: () => api["/api/dashboard/entities"].get().json(),
	});

	const authError = getStatusCode(error) === 401;
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
	return { users: data?.users ?? [], isLoading, error, authError };
};

export const useTeams = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["teams"],
		queryFn: () => api["/api/dashboard/teams"].get().json(),
	});
	return { teams: data?.teams ?? [], isLoading, error };
};

export const useApiKeys = () => {
	const { data, isLoading, error } = useQuery({
		queryKey: ["api_keys"],
		queryFn: () => api["/api/dashboard/api-keys"].get().json(),
	});
	useEffect(() => {
		if (error) createToast("Failed to load API keys", "error");
	}, [error]);
	return { keys: data?.keys ?? [], isLoading, error };
};

export const useDimension = ({
	project,
	dimension,
	metric,
	range,
	filters,
	eventName = "pageview",
	propertyKey,
	comparisonRange,
}: {
	project: ProjectResponse;
	dimension: Dimension;
	propertyKey?: string;
	comparisonRange?: ComparisonRange;
	metric: Metric;
	filters: DimensionFilter[];
	range: DateRange;
	eventName?: string;
}): {
	data: ComparisonRow[] | undefined;
	biggest: number;
	isLoading: boolean;
	error: unknown;
} => {
	const { data, isFetching, isLoading, isPlaceholderData, error } = useQuery({
		placeholderData: (prev, previousQuery) => {
			const key = previousQuery?.queryKey;
			if (
				!prev ||
				key?.[1] !== project.id ||
				key[2] !== dimension ||
				key[3] !== propertyKey ||
				key[4] !== metric ||
				key.at(-1) !== eventName
			) {
				return undefined;
			}
			return !comparisonRange && key[6] !== undefined
				? prev.map((row) => ({ ...row, comparisonValue: undefined }))
				: prev;
		},
		queryKey: [
			"dimension",
			project.id,
			dimension,
			propertyKey,
			metric,
			range.cacheKey(),
			comparisonRange?.cacheKey(),
			filters,
			eventName,
		],
		refetchInterval: range.endsToday() || comparisonRange?.endsToday() ? 60_000 : undefined,
		queryFn: async () => {
			const result = await api["/api/dashboard/project/{project_id}/dimension"]
				.post({
					params: { project_id: project.id },
					json: {
						dimension,
						propertyKey,
						filters,
						metric,
						event: eventName,
						ranges: getReportRanges(range, comparisonRange),
					},
				})
				.json();
			if (typeof result === "string") throw new Error(result);
			const [current, previous] = result.reports;
			return mergeComparisonRows(current, previous);
		},
	});

	return useMemo(() => {
		const biggest = data?.reduce((acc, d) => Math.max(acc, d.value ?? 0, d.comparisonValue ?? 0), 0) ?? 0;
		const sortedData = data?.toSorted(
			(a, b) =>
				Number(a.value == null) - Number(b.value == null) ||
				(metric === "bounce_rate" ? (a.value ?? 0) - (b.value ?? 0) : (b.value ?? 0) - (a.value ?? 0)) ||
				numericCollator.compare(a.dimensionValue, b.dimensionValue),
		);
		return { data: sortedData, biggest, isLoading: isLoading || (isFetching && isPlaceholderData), error };
	}, [data, isFetching, isLoading, isPlaceholderData, error, metric]);
};
export const useProjectGraph = ({
	projectId,
	metric,
	range,
	filters = [],
	eventName = "pageview",
	enabled = true,
	interval: selectedInterval = "auto",
	comparisonRange,
}: {
	projectId?: string;
	metric: Metric;
	range: DateRange;
	filters?: DimensionFilter[];
	eventName?: string;
	enabled?: boolean;
	interval?: GraphInterval | "auto";
	comparisonRange?: ComparisonRange;
}) => {
	let refetchInterval: number | undefined;
	let staleTime = 1000 * 60 * 10;
	if (range.endsToday() || comparisonRange?.endsToday()) {
		refetchInterval = 1000 * 60;
		staleTime = 0;
	}
	const intervalRange = comparisonRange?.fullPeriod ? comparisonRange : range;
	const availableIntervals = intervalRange.getGraphIntervals();
	const autoInterval = range.getGraphInterval();
	const interval =
		selectedInterval !== "auto" && availableIntervals.includes(selectedInterval)
			? selectedInterval
			: availableIntervals.includes(autoInterval)
				? autoInterval
				: intervalRange.getGraphInterval();
	const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
	const queryKey = [
		"project_graph",
		projectId,
		range.cacheKey(),
		comparisonRange?.cacheKey(),
		metric,
		filters,
		interval,
		timezone,
		eventName,
	];

	const {
		data: graphResult,
		isError,
		isLoading,
		isFetching,
		isPlaceholderData,
	} = useQuery({
		refetchInterval,
		staleTime,
		enabled: projectId !== undefined && enabled,
		queryKey,
		queryFn: async () => {
			const result = await api["/api/dashboard/project/{project_id}/graph"]
				.post({
					params: { project_id: projectId ?? "" },
					json: {
						ranges: getReportRanges(range, comparisonRange),
						metric,
						interval,
						timezone,
						filters,
						event: eventName,
					},
				})
				.json();
			if (typeof result === "string") throw new Error(result);
			const [current, previous] = result.reports;
			return {
				data: toDataPoints(current),
				comparisonData: previous && toDataPoints(previous),
				comparisonRange,
				metric,
				range,
				interval,
			};
		},
		placeholderData: (prev, previousQuery) =>
			previousQuery?.queryKey.at(-1) === eventName && previousQuery.queryKey[3] === comparisonRange?.cacheKey()
				? prev
				: undefined,
	});

	const isUpdating = isFetching && isPlaceholderData;

	return {
		availableIntervals,
		graph: graphResult?.data,
		comparisonGraph: graphResult?.comparisonData,
		displayComparisonRange: graphResult?.comparisonRange,
		displayMetric: graphResult?.metric ?? metric,
		displayRange: graphResult?.range ?? range,
		displayInterval: graphResult?.interval ?? interval,
		isLoading,
		isError,
		isUpdating,
	};
};

export const useCurrentVisitors = (
	projectId?: string,
	{ pages = false, enabled = true, refetchInterval = 15_000 } = {},
) => {
	return useQuery({
		queryKey: ["current_visitors", projectId, pages],
		enabled: projectId !== undefined && enabled,
		refetchInterval,
		staleTime: 15_000,
		queryFn: async () => {
			const live = await api["/api/dashboard/project/{project_id}/current-visitors"]
				.get({ params: { project_id: projectId ?? "" }, query: pages ? { pages } : undefined })
				.json();
			if (typeof live === "string") throw new Error(live);
			return live;
		},
	});
};

export const useProjectStats = ({
	projectId,
	range,
	filters = [],
	eventName = "pageview",
	enabled = true,
	comparisonRange,
}: {
	projectId?: string;
	range: DateRange;
	filters?: DimensionFilter[];
	eventName?: string;
	enabled?: boolean;
	comparisonRange?: ComparisonRange;
}) => {
	const {
		data: stats,
		isError,
		isLoading,
		isFetching,
		isPlaceholderData,
	} = useQuery({
		queryKey: ["project_stats", projectId, range.cacheKey(), comparisonRange?.cacheKey(), filters, eventName],
		refetchInterval: range.endsToday() || comparisonRange?.endsToday() ? 60_000 : undefined,

		enabled: projectId !== undefined && enabled,
		queryFn: async () => {
			const result = await api["/api/dashboard/project/{project_id}/stats"]
				.post({
					params: { project_id: projectId ?? "" },
					json: { ranges: getStatsReportRanges(range, comparisonRange), filters, event: eventName },
				})
				.json();
			if (typeof result === "string") throw new Error(result);
			return result;
		},
		placeholderData: (prev, previousQuery) => {
			if (!prev || previousQuery?.queryKey[1] !== projectId || previousQuery?.queryKey.at(-1) !== eventName)
				return undefined;
			const wasComparing = previousQuery.queryKey[3] !== undefined;
			if (range.variant === "allTime" || wasComparing !== Boolean(comparisonRange)) {
				return { ...prev, reports: prev.reports.slice(0, 1) };
			}
			return prev;
		},
	});

	return {
		stats,
		isLoading,
		isUpdating: isFetching && isPlaceholderData,
		isError,
	};
};

export const invalidateProjects = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
export const invalidateEntities = () => queryClient.invalidateQueries({ queryKey: ["entities"] });
export const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["users"] });
export const invalidateTeams = () => queryClient.invalidateQueries({ queryKey: ["teams"] });
export const invalidateApiKeys = () => queryClient.invalidateQueries({ queryKey: ["api_keys"] });
