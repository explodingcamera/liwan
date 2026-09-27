import styles from "./custom-events.module.css";
import cardStyles from "./dimensions/dimensions.module.css";

import { useState } from "react";
import { XIcon, ZoomInIcon } from "lucide-react";

import type { DateRange } from "@/api/ranges";
import { Dialog } from "@/components/ui/dialog";
import { LoadingSpinner } from "@/components/ui/loading";
import type { DimensionFilter, DisplayOverride } from "@/constants";
import { useCustomEvents } from "@/hooks/api";
import { DimensionValueBar } from "./dimensions";

export const CustomEventsCard = ({
	projectId,
	range,
	filters,
	display,
	selectedEvent,
	onSelectEvent,
}: {
	projectId: string;
	range: DateRange;
	filters: DimensionFilter[];
	display: DisplayOverride;
	selectedEvent: string;
	onSelectEvent: (name: string) => void;
}) => {
	const [search, setSearch] = useState("");
	const unsupportedFilter = filters.some(
		(filter) => filter.dimension === "url_entry" || filter.dimension === "url_exit",
	);
	const { data, isLoading, error } = useCustomEvents({
		projectId,
		range,
		filters: unsupportedFilter ? [] : filters,
		enabled: display !== "hide",
	});
	const biggest = data?.rows[0]?.completions ?? 0;
	const rows = data?.rows ?? [];
	const searchTerm = search.toLowerCase();
	const filteredRows = searchTerm ? rows.filter((row) => row.name.toLowerCase().includes(searchTerm)) : rows;

	if (display === "hide" || (display === "auto" && !isLoading && !error && !data?.hasCustomEvents)) return null;

	return (
		<article className={`${cardStyles.card} ${styles.card}`}>
			{(!data || data.rows.length === 0 || error || unsupportedFilter) && <h2>Events</h2>}
			{isLoading && <LoadingSpinner />}
			{error && <p>Could not load custom events.</p>}
			{unsupportedFilter && (data?.hasCustomEvents || display === "show") && (
				<p>Entry and exit page filters are not supported for custom events.</p>
			)}
			{data && !error && !unsupportedFilter && (
				<>
					{data.rows.length === 0 ? (
						<p>No custom events in this range.</p>
					) : (
						<EventsTable
							rows={rows.slice(0, 6)}
							biggest={biggest}
							selectedEvent={selectedEvent}
							onSelectEvent={onSelectEvent}
						/>
					)}
					{rows.length > 0 && (
						<Dialog
							title="Events"
							description="Custom event completions and unique visitor groups."
							hideTitle
							hideDescription
							autoOverflow
							className={styles.detailsModal}
							trigger={
								<button type="button" className={cardStyles.showMore}>
									<ZoomInIcon size={16} />
									Show details
								</button>
							}
						>
							<h2 className={styles.modalTitle}>Events</h2>
							<Dialog.Close className={cardStyles.detailsClose} aria-label="Close dialog">
								<XIcon size={22} />
							</Dialog.Close>
							<input
								type="search"
								placeholder="Search events"
								aria-label="Search events"
								value={search}
								onChange={(event) => setSearch(event.target.value)}
								className={styles.search}
							/>
							<EventsTable
								rows={filteredRows}
								biggest={biggest}
								selectedEvent={selectedEvent}
								onSelectEvent={onSelectEvent}
							/>
							{filteredRows.length === 0 && <p>No matching events.</p>}
							{data.truncated && <p>Showing the top events only.</p>}
						</Dialog>
					)}
				</>
			)}
		</article>
	);
};

type EventRow = { name: string; completions: number; uniques: number };

const EventsTable = ({
	rows,
	biggest,
	selectedEvent,
	onSelectEvent,
}: {
	rows: EventRow[];
	biggest: number;
	selectedEvent: string;
	onSelectEvent: (name: string) => void;
}) => (
	<table className={styles.table}>
		<thead>
			<tr>
				<th scope="col">Events</th>
				<th scope="col">Completions</th>
				<th
					scope="col"
					title="Distinct visitor groups, not individual people. Groups rotate daily; events without visitor metadata count separately."
				>
					Uniques
				</th>
			</tr>
		</thead>
		<tbody>
			{rows.map((row) => (
				<tr key={row.name}>
					<th scope="row" title={row.name}>
						<div className={styles.bar}>
							<DimensionValueBar value={row.completions} biggest={biggest} selected={selectedEvent === row.name}>
								<button
									type="button"
									className={styles.name}
									aria-pressed={selectedEvent === row.name}
									onClick={() => onSelectEvent(selectedEvent === row.name ? "pageview" : row.name)}
								>
									{row.name}
								</button>
							</DimensionValueBar>
						</div>
					</th>
					<td>{row.completions.toLocaleString()}</td>
					<td>{row.uniques.toLocaleString()}</td>
				</tr>
			))}
		</tbody>
	</table>
);
