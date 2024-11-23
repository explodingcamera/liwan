import styles from "./range.module.css";

import { useRef } from "react";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";

import { cls } from "../../utils";
import { Dialog } from "../dialog";
import { DatePickerRange } from "../daterange";
import { DateRange, wellKnownRanges, type RangeName } from "../../api/ranges";
import { api, useQuery } from "../../api";
import { endOfDay, startOfDay } from "date-fns";

export const SelectRange = ({
	onSelect,
	range,
	projectId,
}: { onSelect: (range: DateRange) => void; range: DateRange; projectId?: string }) => {
	const detailsRef = useRef<HTMLDetailsElement>(null);

	const handleSelect = (range: DateRange) => () => {
		console.log("range", range, detailsRef.current);
		if (detailsRef.current) detailsRef.current.open = false;
		onSelect(range);
	};

	const allTime = useQuery({
		queryKey: ["allTime", projectId],
		enabled: !!projectId,
		staleTime: 7 * 24 * 60 * 60 * 1000,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/earliest"].get({ params: { project_id: projectId || "" } }).json(),
	});

	const selectAllTime = async () => {
		if (!projectId) return;
		if (!allTime.data) return;
		const from = new Date(allTime.data);
		const range = new DateRange({ start: startOfDay(from), end: endOfDay(new Date()) });
		range.variant = "allTime";
		onSelect(range);
		if (detailsRef.current) detailsRef.current.open = false;
	};

	return (
		<div className={styles.container}>
			<button type="button" className="secondary" onClick={handleSelect(range.previous())}>
				<ChevronLeftIcon size="24" />
			</button>
			<button type="button" className="secondary" onClick={handleSelect(range.next())}>
				<ChevronRightIcon size="24" />
			</button>
			<details ref={detailsRef} className={cls("dropdown", styles.selectRange)}>
				<summary>{range.format()}</summary>
				<ul>
					{Object.entries(wellKnownRanges).map(([key, value]) => (
						<li key={key}>
							<button
								type="button"
								className={key === range.serialize() ? styles.selected : ""}
								onClick={handleSelect(new DateRange(key as RangeName))}
							>
								{value}
							</button>
						</li>
					))}
					{projectId && allTime.data && (
						<li>
							<button
								type="button"
								className={range.variant === "allTime" ? styles.selected : ""}
								onClick={selectAllTime}
							>
								All Time
							</button>
						</li>
					)}
					<li>
						<Dialog
							className={styles.rangeDialog}
							description="Select a custom date range."
							hideDescription
							trigger={
								<button type="button" className={range.isCustom() ? styles.selected : ""}>
									Custom
								</button>
							}
							onOpenChange={(open) => {
								if (open && detailsRef.current) detailsRef.current.open = false;
							}}
							title="Custom Range"
							showClose
							hideTitle
							autoOverflow
						>
							<DatePickerRange onSelect={(range) => handleSelect(range)()} />
						</Dialog>
					</li>
				</ul>
			</details>
		</div>
	);
};
