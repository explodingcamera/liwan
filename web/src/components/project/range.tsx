import styles from "./range.module.css";
import { useRef } from "react";
import { deserializeRange, nextRange, previusRange, rangeNames, type RangeName } from "../../api/ranges";
import { cls } from "../../utils";
import { Dialog } from "../dialog";
import { DateRange } from "../daterange";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";
import { formatDateRange } from "little-date";

export const SelectRange = ({ onSelect, range }: { onSelect: (name: string) => void; range: string }) => {
	const rangeName = range.includes(":") ? "Custom" : rangeNames[range as RangeName];
	const detailsRef = useRef<HTMLDetailsElement>(null);

	const handleSelect = (name: string) => () => {
		if (detailsRef.current) detailsRef.current.open = false;
		onSelect(name);
	};

	const isCustom = !Object.keys(rangeNames).includes(range);
	const r = deserializeRange(range);

	return (
		<div className={styles.container}>
			<button type="button" className="secondary" onClick={handleSelect(previusRange(range))}>
				<ChevronLeftIcon size="24" />
			</button>
			<button type="button" className="secondary" onClick={handleSelect(nextRange(range))}>
				<ChevronRightIcon size="24" />
			</button>
			<details ref={detailsRef} className={cls("dropdown", styles.selectRange)}>
				<summary>{isCustom ? formatDateRange(new Date(r.start), new Date(r.end)) : rangeName}</summary>
				<ul>
					{Object.entries(rangeNames).map(([key, value]) => (
						<li key={key}>
							{/* biome-ignore lint/a11y/useValidAnchor: this is fine */}
							<button
								type="button"
								className={key === range ? styles.selected : ""}
								onClick={handleSelect(key as RangeName)}
							>
								{value}
							</button>
						</li>
					))}
					<li>
						<Dialog
							trigger={
								<button type="button" className={isCustom ? styles.selected : ""}>
									Custom
								</button>
							}
							title="Custom Range"
							showClose
							hideTitle
							autoOverflow
						>
							<DateRange />
						</Dialog>
					</li>
				</ul>
			</details>
		</div>
	);
};
