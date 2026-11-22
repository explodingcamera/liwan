import cardStyles from "./dimensions/dimensions.module.css";

import { useDeferredValue, useState } from "react";
import { XIcon, ZapIcon, ZoomInIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { useCustomEvents } from "@/hooks/api";
import { cls, fuzzyFilter } from "@/utils";
import type { ProjectQuery } from ".";
import { DimensionValueBar, DimensionValueButton } from "./dimensions";
import { DimensionMetricValue } from "./dimensions/metric-value";
import { ReportPagination, useReportPagination } from "./dimensions/pagination";
import { formatMetricVal } from "./metric-format";

type EventRow = {
	name: string;
	completions?: number | null;
	uniques?: number | null;
	comparisonCompletions?: number | null;
	comparisonUniques?: number | null;
};

const uniquesTitle =
	"Distinct visitor groups, not individual people. Groups rotate daily; events without visitor metadata count separately.";

const EventsHeader = ({ children }: { children?: React.ReactNode }) => (
	<div className={cardStyles.dimensionHeader}>
		<div>Events</div>
		<div className={cls(cardStyles.metricColumn, cardStyles.metricHeading)}>Completions</div>
		<div className={cls(cardStyles.metricColumn, cardStyles.metricHeading)} title={uniquesTitle}>
			Uniques
		</div>
		{children}
	</div>
);

export const CustomEventsCard = ({
	query,
	onSelectEvent,
}: {
	query: ProjectQuery;
	onSelectEvent: (name: string) => void;
}) => {
	const [search, setSearch] = useState("");
	const deferredSearch = useDeferredValue(search);
	const unsupportedFilter = query.filters.some(
		(filter) => filter.dimension === "url_entry" || filter.dimension === "url_exit",
	);
	const events = useCustomEvents({
		projectId: query.project.id,
		range: query.range,
		filters: query.filters,
		enabled: !query.project.customEventsHidden && !unsupportedFilter,
		comparisonRange: query.comparisonRange,
	});
	const rows = (events.data?.rows ?? []).filter(
		(row) => query.eventName === "pageview" || row.name === query.eventName,
	);
	const searchResults = deferredSearch ? fuzzyFilter(deferredSearch, rows, ["name"]) : rows;
	const { pageRows, hasPages, previous, next } = useReportPagination(rows, query);

	if (query.project.customEventsHidden) return null;

	const empty = unsupportedFilter
		? "Entry and exit page filters are not supported for custom events."
		: events.error
			? "Could not load custom events."
			: !events.isLoading && rows.length === 0
				? "No custom events in this range"
				: undefined;
	const biggest = rows.reduce((max, row) => Math.max(max, row.completions ?? 0, row.comparisonCompletions ?? 0), 0);
	const renderRow = (row: EventRow) => (
		<div
			key={row.name}
			className={cardStyles.dimensionRow}
			title={
				row.comparisonCompletions === undefined
					? undefined
					: `Comparison: ${formatMetricVal(row.comparisonCompletions, "views")} completions, ${formatMetricVal(row.comparisonUniques, "unique_visitors")} uniques`
			}
		>
			<DimensionValueBar
				value={row.completions}
				comparisonValue={row.comparisonCompletions}
				biggest={biggest}
				metric="views"
				selected={query.eventName === row.name}
			>
				<ZapIcon size={16} />
				<DimensionValueButton onSelect={() => onSelectEvent(query.eventName === row.name ? "pageview" : row.name)}>
					{row.name}
				</DimensionValueButton>
			</DimensionValueBar>
			<DimensionMetricValue
				className={cardStyles.metricColumn}
				value={row.completions}
				comparisonValue={row.comparisonCompletions}
				metric="views"
			/>
			<DimensionMetricValue
				className={cardStyles.metricColumn}
				value={row.uniques}
				comparisonValue={row.comparisonUniques}
				metric="unique_visitors"
			/>
		</div>
	);

	return (
		<article className={cardStyles.card}>
			<EventsHeader />
			<div className={cardStyles.dimensionTable} aria-busy={events.isLoading}>
				{events.isLoading && (
					<div className={cardStyles.loadingOverlay} data-no-delay={!events.data}>
						<LoadingSpinner immediate />
					</div>
				)}
				{empty ? (
					<div className={cardStyles.dimensionEmpty}>
						<div>{empty}</div>
					</div>
				) : (
					pageRows?.map(renderRow)
				)}
			</div>
			<div className={cardStyles.cardFooter}>
				<Dialog
					title="Events"
					description="Custom event completions and unique visitor groups."
					hideTitle
					hideDescription
					autoOverflow
					className={cardStyles.detailsModal}
					trigger={
						<button type="button" className={cls(cardStyles.showMore, empty && cardStyles.showMoreHidden)}>
							<ZoomInIcon size={16} />
							Show details
						</button>
					}
				>
					<div className={cardStyles.dimensionTable} style={{ "--count": searchResults.length } as React.CSSProperties}>
						<EventsHeader>
							<Dialog.Close className={cardStyles.detailsClose} aria-label="Close dialog">
								<XIcon size={20} />
							</Dialog.Close>
						</EventsHeader>
						<input
							type="search"
							placeholder="Search"
							value={search}
							onChange={(event) => setSearch(event.target.value)}
							className={cardStyles.search}
						/>
						<div className={cardStyles.detailRows} aria-busy={events.isLoading}>
							{searchResults.map(renderRow)}
							{events.isLoading && (
								<div className={cardStyles.loadingOverlay} data-no-delay={!events.data}>
									<LoadingSpinner immediate />
								</div>
							)}
							{events.data?.truncated && <div className={cardStyles.dimensionEmpty}>Showing the top events only.</div>}
						</div>
					</div>
				</Dialog>
				{hasPages && !empty && <ReportPagination previous={previous} next={next} />}
			</div>
		</article>
	);
};
