import type { NormalizeOAS, OASModel } from "fets";
import type dashboardspec from "./dashboard";

export type DashboardSpec = NormalizeOAS<typeof dashboardspec>;
export type Metric = OASModel<DashboardSpec, "Metric">;
export type Dimension = OASModel<DashboardSpec, "Dimension">;
export type DimensionTableRow = OASModel<DashboardSpec, "DimensionTableRow">;
export type DateRange = OASModel<DashboardSpec, "DateRange">;
export type ProjectResponse = OASModel<DashboardSpec, "ProjectResponse">;
export type EntityResponse = OASModel<DashboardSpec, "EntityResponse">;
export type UserResponse = OASModel<DashboardSpec, "UserResponse">;
export type StatsResponse = OASModel<DashboardSpec, "StatsResponse">;
