import menuStyles from "@/components/ui/menu.module.css";
import styles from "./range.module.css";

import { useState } from "react";
import { Menu } from "@base-ui/react/menu";
import { ChevronDownIcon } from "lucide-react";

import type { DateRange } from "@/api/ranges";
import { Dialog } from "@/components/ui/dialog";
import { formatDateRange } from "@/utils";
import { type Comparison, type ComparisonPreset, getComparisonOptions, getComparisonRange } from "./comparison";
import { DatePickerRange } from "./date-range";
import { RangeMenu } from "./range-menu";

export const SelectComparison = ({
	range,
	comparison,
	onChange,
}: {
	range: DateRange;
	comparison: Comparison;
	onChange: (comparison: Comparison) => void;
}) => {
	const [customOpen, setCustomOpen] = useState(false);
	const comparisonRange = getComparisonRange(range, comparison);
	if (!comparisonRange || comparison.mode === "off") return null;
	const dateLabel = formatDateRange(comparisonRange.value.start, comparisonRange.value.end);
	const periodLabel = comparisonRange.format();
	const options = getComparisonOptions(range);
	const selectedOption = options.find(
		(option) =>
			option.range.value.start.getTime() === comparisonRange.value.start.getTime() &&
			option.range.value.end.getTime() === comparisonRange.value.end.getTime(),
	);
	const selectedMode = comparison.mode === "custom" ? "custom" : selectedOption?.mode;
	return (
		<div className={styles.comparison}>
			<RangeMenu
				className={styles.compareRange}
				ariaLabel={`Comparison period: vs ${periodLabel}`}
				title={dateLabel}
				label={
					<>
						<span className={styles.comparisonLabel}>
							<span className={styles.vs}>vs</span>
							<span>{periodLabel}</span>
						</span>
						<ChevronDownIcon size={14} aria-hidden="true" />
					</>
				}
			>
				{options.map(({ mode, label }) => (
					<Menu.Item
						key={mode}
						className={menuStyles.item}
						data-selected={selectedMode === mode ? "true" : undefined}
						onClick={() => onChange({ mode })}
					>
						{label}
					</Menu.Item>
				))}
				<Menu.Item
					className={menuStyles.item}
					data-selected={comparison.mode === "custom" ? "true" : undefined}
					onClick={() => setCustomOpen(true)}
				>
					Custom date…
				</Menu.Item>
				<Menu.Separator className={menuStyles.separator} />
				<Menu.Item className={menuStyles.item} onClick={() => onChange({ mode: "off" })}>
					Remove comparison
				</Menu.Item>
			</RangeMenu>
			<select
				className={styles.mobileComparison}
				aria-label="Comparison period"
				title={dateLabel}
				value="selected"
				onChange={(event) => {
					const mode = event.currentTarget.value;
					if (mode === "choose-custom") setCustomOpen(true);
					else if (mode !== "selected") onChange({ mode: mode as "off" | ComparisonPreset });
				}}
			>
				<option value="selected" disabled hidden>
					vs {periodLabel}
				</option>
				{options.map(({ mode, label }) => (
					<option key={mode} value={mode}>
						{label}
					</option>
				))}
				<option value="choose-custom">Custom date…</option>
				<option value="off">Remove comparison</option>
			</select>
			<Dialog
				title="Compare from"
				description="Choose a start date. The comparison matches this report’s length."
				hideDescription
				className={styles.comparisonDialog}
				open={customOpen}
				onOpenChange={setCustomOpen}
				autoOverflow
			>
				<DatePickerRange
					startOnly
					initialRange={comparisonRange}
					onSelect={(selected) => {
						onChange({ mode: "custom", start: selected.value.start });
						setCustomOpen(false);
					}}
				/>
			</Dialog>
		</div>
	);
};
