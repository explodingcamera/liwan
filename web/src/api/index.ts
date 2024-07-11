import { createClient, type NormalizeOAS, type OASModel } from "fets";
export { queryClient, useMutation, useQuery } from "./utils";
import type dashboardspec from "./dashboard";
import { useQuery } from "./utils";

export type DashboardSpec = NormalizeOAS<typeof dashboardspec>;
export type Metric = OASModel<DashboardSpec, "Metric">;
export type DateRange = OASModel<DashboardSpec, "DateRange">;

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
	avg_views_per_session: "Avg Views Per Session",
};

export const useMe = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["me"],
		staleTime: 60 * 1000, // 1 minute
		queryFn: () => api["/api/dashboard/auth/me"].get().json(),
	});
	return { role: data?.role, username: data?.username, isLoading };
};
