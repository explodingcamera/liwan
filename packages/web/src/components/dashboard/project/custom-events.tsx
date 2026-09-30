import cardStyles from "./dimensions/dimensions.module.css";

import { useDeferredValue, useState } from "react";
import { XIcon, ZapIcon, ZoomInIcon } from "lucide-react";

import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import { useCustomEvents } from "@/hooks/api";
import { cls, formatMetricVal, fuzzyFilter } from "@/utils";
import type { ProjectQuery } from ".";
import { DimensionValueBar, DimensionValueButton } from "./dimensions";

type EventRow = { name: string; completions: number; uniques: number };

const uniquesTitle =
	"Distinct visitor groups, not individual people. Groups rotate daily; events without visitor metadata count separately.";

const EventsHeader = ({ children }: { children?: React.ReactNode }) => (
	<div className={cardStyles.dimensionHeader}>
		<div>Events</div>
		<div className={cardStyles.metricColumn}>Completions</div>
		<div className={cardStyles.metricColumn} title={uniquesTitle}>
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
		filters: unsupportedFilter ? [] : query.filters,
		enabled: !query.project.customEventsHidden,
	});
	const rows = (events.data?.rows ?? []).filter(
		(row) => query.eventName === "pageview" || row.name === query.eventName,
	);
	const searchResults = deferredSearch ? fuzzyFilter(deferredSearch, rows, ["name"]) : rows;

	if (query.project.customEventsHidden) return null;

	const empty = events.error
		? "No data available"
		: unsupportedFilter
			? "Entry and exit page filters are not supported for custom events."
			: !events.isLoading && rows.length === 0
				? "No custom events in this range"
				: undefined;
	const biggest = rows[0]?.completions ?? 0;
	const renderRow = (row: EventRow) => (
		<div key={row.name} className={cardStyles.dimensionRow}>
			<DimensionValueBar value={row.completions} biggest={biggest} selected={query.eventName === row.name}>
				<ZapIcon size={16} />
				<DimensionValueButton onSelect={() => onSelectEvent(query.eventName === row.name ? "pageview" : row.name)}>
					{row.name}
				</DimensionValueButton>
			</DimensionValueBar>
			<div className={cardStyles.metricColumn}>{formatMetricVal(row.completions, "views")}</div>
			<div className={cardStyles.metricColumn}>{formatMetricVal(row.uniques, "unique_visitors")}</div>
		</div>
	);

	return (
		<article className={cardStyles.card}>
			<EventsHeader />
			<div className={cardStyles.dimensionTable} style={{ "--count": 6 } as React.CSSProperties}>
				{events.isLoading && (
					<div className={cardStyles.loadingOverlay} data-no-delay={rows.length === 0}>
						<LoadingSpinner immediate />
					</div>
				)}
				{empty ? (
					<div className={cardStyles.dimensionEmpty}>
						<div>{empty}</div>
					</div>
				) : (
					rows.slice(0, 6).map(renderRow)
				)}
			</div>
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
					{searchResults.map(renderRow)}
					{events.data?.truncated && <div className={cardStyles.dimensionEmpty}>Showing the top events only.</div>}
				</div>
			</Dialog>
		</article>
	);
};
