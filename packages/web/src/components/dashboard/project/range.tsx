import menuStyles from "@/components/ui/menu.module.css";
import styles from "./range.module.css";

import { useEffect, useState } from "react";
import { Menu } from "@base-ui/react/menu";
import { endOfDay, startOfDay } from "date-fns";
import { ChevronDownIcon, ChevronLeftIcon, ChevronRightIcon } from "lucide-react";

import { api, useQuery } from "@/api";
import type { RangeName } from "@/api/ranges";
import { DateRange, isRangeName, wellKnownRanges } from "@/api/ranges";
import { Dialog } from "@/components/ui/dialog";
import { cls, formatDateRange } from "@/utils";
import type { Comparison } from "./comparison";
import { SelectComparison } from "./comparison-picker";
import { DatePickerRange } from "./date-range";
import { RangeMenu } from "./range-menu";

export const SelectRange = ({
	onSelect,
	range,
	projectId,
	comparison,
	onCompare,
}: {
	onSelect: (range: DateRange) => void;
	range: DateRange;
	projectId?: string;
	comparison?: Comparison;
	onCompare?: (comparison: Comparison) => void;
}) => {
	const [customOpen, setCustomOpen] = useState(false);

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (
				event.defaultPrevented ||
				event.repeat ||
				event.altKey ||
				event.ctrlKey ||
				event.metaKey ||
				event.shiftKey ||
				(event.key !== "ArrowLeft" && event.key !== "ArrowRight")
			) {
				return;
			}

			const target = event.target;
			if (
				target instanceof Element &&
				target.closest("a, button, input, select, summary, textarea, [contenteditable='true'], [role='button']")
			) {
				return;
			}

			event.preventDefault();
			onSelect(event.key === "ArrowLeft" ? range.previous() : range.next());
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [onSelect, range]);

	const allTime = useQuery({
		queryKey: ["allTime", projectId],
		enabled: !!projectId,
		staleTime: 7 * 24 * 60 * 60 * 1000,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/earliest"].get({ params: { project_id: projectId || "" } }).json(),
	});
	const selectAllTime = () => {
		if (!projectId) return;
		if (!allTime.data?.earliest) return;
		const from = new Date(allTime.data.earliest);
		const range = new DateRange({
			start: startOfDay(from),
			end: endOfDay(new Date()),
		});
		range.variant = "allTime";
		onCompare?.({ mode: "off" });
		onSelect(range);
	};
	const selectMobileRange = (value: string) => {
		if (value === "compare") onCompare?.({ mode: "previous" });
		else if (value === "custom") setCustomOpen(true);
		else if (value === "allTime") selectAllTime();
		else if (isRangeName(value)) onSelect(new DateRange(value));
	};
	const { start, end } = range.value;
	const dateTitle = range.isCustom() ? formatDateRange(start, end) : undefined;
	const canCompare = range.variant !== "allTime" && onCompare !== undefined;
	const isComparing = canCompare && comparison !== undefined && comparison.mode !== "off";

	return (
		<div className={styles.container}>
			<button
				type="button"
				className={cls("button-ghost", styles.stepButton)}
				aria-label="Previous date range"
				aria-keyshortcuts="ArrowLeft"
				onClick={() => onSelect(range.previous())}
			>
				<ChevronLeftIcon size="24" />
			</button>
			<button
				type="button"
				className={cls("button-ghost", styles.stepButton)}
				aria-label="Next date range"
				aria-keyshortcuts="ArrowRight"
				onClick={() => onSelect(range.next())}
			>
				<ChevronRightIcon size="24" />
			</button>
			<div className={styles.rangeField} data-comparing={isComparing}>
				<div className={styles.desktopRange}>
					<RangeMenu
						title={dateTitle}
						label={
							<>
								<span>{range.format()}</span>
								{isComparing && <ChevronDownIcon size={14} aria-hidden="true" />}
							</>
						}
					>
						{Object.entries(wellKnownRanges).map(([key, value]) => (
							<Menu.Item
								key={key}
								className={menuStyles.item}
								data-selected={key === range.serialize() ? "true" : undefined}
								onClick={() => onSelect(new DateRange(key as RangeName))}
							>
								{value}
							</Menu.Item>
						))}
						{projectId && allTime.data && (
							<Menu.Item
								className={menuStyles.item}
								data-selected={range.variant === "allTime" ? "true" : undefined}
								onClick={selectAllTime}
							>
								All Time
							</Menu.Item>
						)}
						<Menu.Item
							className={menuStyles.item}
							data-selected={range.isCustom() ? "true" : undefined}
							onClick={() => setCustomOpen(true)}
						>
							Custom
						</Menu.Item>
						{canCompare && comparison?.mode === "off" && (
							<>
								<Menu.Separator className={menuStyles.separator} />
								<Menu.Item className={menuStyles.item} onClick={() => onCompare({ mode: "previous" })}>
									Compare
								</Menu.Item>
							</>
						)}
					</RangeMenu>
				</div>
				<select
					className={styles.mobileRange}
					aria-label="Date range"
					title={dateTitle}
					value={range.isCustom() ? "custom-range" : range.variant === "allTime" ? "allTime" : range.serialize()}
					onChange={(event) => selectMobileRange(event.currentTarget.value)}
				>
					{Object.entries(wellKnownRanges).map(([key, value]) => (
						<option key={key} value={key}>
							{value}
						</option>
					))}
					{projectId && (allTime.data || range.variant === "allTime") && <option value="allTime">All Time</option>}
					{range.isCustom() && <option value="custom-range">{range.format()}</option>}
					<option value="custom">Custom...</option>
					{canCompare && comparison?.mode === "off" && <option value="compare">Compare</option>}
				</select>
				{isComparing && <SelectComparison range={range} comparison={comparison} onChange={onCompare} />}
			</div>
			<Dialog
				className={styles.rangeDialog}
				description="Choose a start and end date for the report."
				hideDescription
				open={customOpen}
				onOpenChange={setCustomOpen}
				title="Custom range"
				autoOverflow
			>
				<DatePickerRange
					initialRange={range}
					onSelect={(value) => {
						onSelect(value);
						setCustomOpen(false);
					}}
				/>
			</Dialog>
		</div>
	);
};
