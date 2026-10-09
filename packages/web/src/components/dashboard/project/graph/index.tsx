import menuStyles from "@/components/ui/menu.module.css";
import styles from "./linegraph.module.css";

import { lazy, Suspense } from "react";
import { Menu } from "@base-ui/react/menu";
import { CalendarDaysIcon } from "lucide-react";

import type { DateRange } from "@/api/ranges.ts";
import { LoadingSpinner } from "@/components/ui/loading";
import type { GraphInterval, GraphResponse, Metric } from "@/constants.ts";

const LineGraphInner = lazy(() => import("./linegraph.tsx").then(({ LineGraph }) => ({ default: LineGraph })));
const intervalLabels = {
	auto: "Auto",
	hour: "Hourly",
	day: "Daily",
	week: "Weekly",
	month: "Monthly",
};

export const LineGraph = ({
	isLoading,
	isUpdating,
	data,
	title,
	metric,
	range,
	interval = range.getGraphInterval(),
	selectedInterval = "auto",
	onSelectInterval,
	availableIntervals,
	onSelectRange,
	onUndoRange,
}: {
	data?: DataPoint[];
	isLoading?: boolean;
	isUpdating?: boolean;
	title: string;
	metric: Metric;
	range: DateRange;
	interval?: GraphInterval;
	selectedInterval?: GraphInterval | "auto";
	onSelectInterval?: (interval: GraphInterval | "auto") => void;
	availableIntervals?: GraphInterval[];
	onSelectRange?: (range: DateRange) => void;
	onUndoRange?: () => void;
}) => {
	const loading = isLoading || isUpdating;
	const activeInterval = availableIntervals?.includes(selectedInterval as GraphInterval) ? selectedInterval : "auto";

	return (
		<div className={styles.graphContainer}>
			{onSelectInterval && availableIntervals && (
				<Menu.Root>
					<Menu.Trigger
						className={styles.intervalButton}
						aria-label={`Graph resolution: ${intervalLabels[activeInterval]}`}
						title={`Graph resolution: ${intervalLabels[activeInterval]}`}
					>
						<CalendarDaysIcon size={18} aria-hidden="true" />
						<span>{intervalLabels[activeInterval]}</span>
					</Menu.Trigger>
					<Menu.Portal>
						<Menu.Positioner className={menuStyles.positioner} align="start" sideOffset={4}>
							<Menu.Popup className={`${menuStyles.popup} ${styles.intervalPopup}`}>
								{(["auto", ...availableIntervals] as const).map((option) => (
									<Menu.Item
										key={option}
										className={`${menuStyles.item} ${styles.intervalItem}`}
										data-selected={option === activeInterval ? "true" : undefined}
										onClick={() => onSelectInterval(option)}
									>
										{intervalLabels[option]}
									</Menu.Item>
								))}
							</Menu.Popup>
						</Menu.Positioner>
					</Menu.Portal>
				</Menu.Root>
			)}
			{loading && (
				<div
					className={styles.updatingOverlay}
					role="status"
					aria-busy="true"
					aria-label={isLoading ? "Loading graph" : "Updating graph"}
					data-no-delay={isLoading}
				>
					<LoadingSpinner immediate />
				</div>
			)}
			<Suspense
				fallback={
					loading ? null : (
						<div
							className={styles.updatingOverlay}
							role="status"
							aria-busy="true"
							aria-label="Loading graph"
							data-no-delay="true"
						>
							<LoadingSpinner immediate />
						</div>
					)
				}
			>
				<LineGraphInner
					data={data ?? []}
					title={title}
					metric={metric}
					range={range}
					interval={interval}
					onSelectRange={onSelectRange}
					onUndoRange={onUndoRange}
				/>
			</Suspense>
		</div>
	);
};

export type DataPoint = {
	x: Date;
	y: number;
};

export const toDataPoints = (data: GraphResponse["data"]): DataPoint[] => {
	return data.map((point) => ({
		x: new Date(point.binStart),
		y: point.value,
	}));
};
