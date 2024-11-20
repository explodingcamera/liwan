import styles from "./range.module.css";

import { useRef } from "react";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";

import { cls } from "../../utils";
import { Dialog } from "../dialog";
import { DatePickerRange } from "../daterange";
import { DateRange, wellKnownRanges, type RangeName } from "../../api/ranges";

export const SelectRange = ({ onSelect, range }: { onSelect: (range: DateRange) => void; range: DateRange }) => {
	const detailsRef = useRef<HTMLDetailsElement>(null);

	const handleSelect = (range: DateRange) => () => {
		if (detailsRef.current) detailsRef.current.open = false;
		onSelect(range);
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
					<li>
						<Dialog
							className={styles.rangeDialog}
							trigger={
								<button type="button" className={range.isCustom() ? styles.selected : ""}>
									Custom
								</button>
							}
							title="Custom Range"
							showClose
							hideTitle
							autoOverflow
						>
							<DatePickerRange />
						</Dialog>
					</li>
				</ul>
			</details>
		</div>
	);
};
