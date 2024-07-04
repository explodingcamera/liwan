import { createClient, type NormalizeOAS, type OASModel } from "fets";
export { queryClient, useMutation, useQuery } from "./utils";
import type dashboardspec from "./dashboard";

export type DashboardSpec = NormalizeOAS<typeof dashboardspec>;
export type Metric = OASModel<DashboardSpec, "Metric">;
export type DateRange = OASModel<DashboardSpec, "DateRange">;

export const api = createClient<DashboardSpec>({
	globalParams: { credentials: "same-origin" },
});

export const metricNames: Record<Metric, string> = {
	views: "Total Views",
	sessions: "Total Sessions",
	unique_visitors: "Unique Visitors",
	avg_views_per_session: "Avg Views Per Session",
};
