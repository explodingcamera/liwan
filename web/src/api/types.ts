import type { NormalizeOAS, OASModel } from "fets";
import type dashboardspec from "./dashboard";

export type DashboardSpec = NormalizeOAS<typeof dashboardspec>;
export type Metric = OASModel<DashboardSpec, "Metric">;
export type DateRange = OASModel<DashboardSpec, "DateRange">;
export type Dimension = OASModel<DashboardSpec, "Dimension">;
export type DimensionFilter = OASModel<DashboardSpec, "DimensionFilter">;
export type DimensionTableRow = OASModel<DashboardSpec, "DimensionTableRow">;
export type FilterType = OASModel<DashboardSpec, "FilterType">;

export const dimensions = [
	"platform",
	"browser",
	"url",
	"path",
	"mobile",
	"referrer",
	"city",
	"country",
	"fqdn",
] as const satisfies Dimension[];

export const filterTypes = [
	"contains",
	"equal",
	"is_null",
	"ends_with",
	"is_false",
	"is_true",
	"starts_with",
] as const satisfies FilterType[];

export type ProjectResponse = OASModel<DashboardSpec, "ProjectResponse">;
export type EntityResponse = OASModel<DashboardSpec, "EntityResponse">;
export type UserResponse = OASModel<DashboardSpec, "UserResponse">;
export type StatsResponse = OASModel<DashboardSpec, "StatsResponse">;
