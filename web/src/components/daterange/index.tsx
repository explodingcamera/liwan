import { endOfDay, startOfDay, subWeeks } from "date-fns";
import { useId, useRef, useState } from "react";
import { DateRange } from "../../api/ranges";
import { Dialog } from "../dialog";
import styles from "./daterange.module.css";

export const DatePickerRange = ({ onSelect }: { onSelect: (range: DateRange) => void }) => {
	const [start, setStart] = useState(() => toHtmlDate(subWeeks(new Date(), 1)));
	const [end, setEnd] = useState(() => toHtmlDate(new Date()));
	const closeRef = useRef<HTMLButtonElement>(null);

	const handleSelect = () => {
		onSelect(new DateRange({ start: startOfDay(start), end: endOfDay(end) }));
		closeRef.current?.click();
	};

	const startId = useId();
	const endId = useId();

	return (
		<div className={styles.container}>
			<label htmlFor={startId}>
				Start date:
				<input
					type="date"
					id={startId}
					name="trip-start"
					min="1997-01-01"
					max="2030-12-31"
					value={start}
					onChange={(e) => setStart(e.target.value)}
				/>
			</label>
			<label htmlFor={endId}>
				End date:
				<input
					type="date"
					id={endId}
					name="trip-start"
					min="1997-01-01"
					max="2030-12-31"
					value={end}
					onChange={(e) => setEnd(e.target.value)}
				/>
			</label>

			<div>
				<Dialog.Close asChild ref={closeRef}>
					<button className="secondary outline" type="button">
						Cancel
					</button>
				</Dialog.Close>

				<button type="button" className="secondary" onClick={handleSelect}>
					Select
				</button>
			</div>
		</div>
	);
};

const toHtmlDate = (date: Date) => date.toISOString().split("T")[0];
