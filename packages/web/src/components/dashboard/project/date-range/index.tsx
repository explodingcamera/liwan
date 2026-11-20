import styles from "./date-range.module.css";

import { useId, useState } from "react";
import { endOfDay, format, isValid, parseISO, subWeeks } from "date-fns";

import { DateRange } from "@/api/ranges";
import { Dialog } from "@/components/ui/dialog";
import { cls } from "@/utils";

export const DatePickerRange = ({
	onSelect,
	initialRange,
	startOnly = false,
}: {
	onSelect: (range: DateRange) => void;
	initialRange?: DateRange;
	startOnly?: boolean;
}) => {
	const [start, setStart] = useState(() => format(initialRange?.value.start ?? subWeeks(new Date(), 1), "yyyy-MM-dd"));
	const [end, setEnd] = useState(() => format(initialRange?.value.end ?? new Date(), "yyyy-MM-dd"));

	const startId = useId();
	const endId = useId();

	return (
		<form
			className={cls(styles.container, startOnly && styles.singleDate)}
			onSubmit={(event) => {
				event.preventDefault();
				const from = parseISO(start);
				const through = endOfDay(parseISO(startOnly ? start : end));
				if (!isValid(from) || !isValid(through) || from > through) return;
				onSelect(new DateRange({ start: from, end: through }));
			}}
		>
			<label htmlFor={startId}>
				Start date
				<input
					type="date"
					id={startId}
					name="startDate"
					min="1997-01-01"
					required
					max={startOnly ? undefined : end || undefined}
					value={start}
					onChange={(e) => setStart(e.target.value)}
				/>
			</label>
			{!startOnly && (
				<label htmlFor={endId}>
					End date
					<input
						type="date"
						id={endId}
						name="endDate"
						min={start || "1997-01-01"}
						required
						value={end}
						onChange={(e) => setEnd(e.target.value)}
					/>
				</label>
			)}
			{startOnly && <p className={styles.hint}>Same length as the selected range.</p>}

			<div className={styles.actions}>
				<Dialog.Close className="button-secondary">Cancel</Dialog.Close>

				<button type="submit" disabled={!start || (!startOnly && (!end || start > end))}>
					Apply
				</button>
			</div>
		</form>
	);
};
