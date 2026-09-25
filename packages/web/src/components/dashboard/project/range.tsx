import menuStyles from "@/components/ui/menu.module.css";
import styles from "./range.module.css";

import { useEffect, useState } from "react";
import { Menu } from "@base-ui/react/menu";
import { endOfDay, startOfDay } from "date-fns";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";

import { api, useQuery } from "@/api";
import type { RangeName } from "@/api/ranges";
import { DateRange, wellKnownRanges } from "@/api/ranges";
import { Dialog } from "@/components/ui/dialog";
import { cls } from "@/utils";
import { DatePickerRange } from "./date-range";

export const SelectRange = ({
	onSelect,
	range,
	projectId,
}: {
	onSelect: (range: DateRange) => void;
	range: DateRange;
	projectId?: string;
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

	const handleSelect = (range: DateRange) => () => onSelect(range);

	const allTime = useQuery({
		queryKey: ["allTime", projectId],
		enabled: !!projectId,
		staleTime: 7 * 24 * 60 * 60 * 1000,
		queryFn: () =>
			api["/api/dashboard/project/{project_id}/earliest"].get({ params: { project_id: projectId || "" } }).json(),
	});
	const selectAllTime = async () => {
		if (!projectId) return;
		if (!allTime.data?.earliest) return;
		const from = new Date(allTime.data.earliest);
		const range = new DateRange({
			start: startOfDay(from),
			end: endOfDay(new Date()),
		});
		range.variant = "allTime";
		onSelect(range);
	};
	const selectMobileRange = (value: string) => {
		if (value === "custom") setCustomOpen(true);
		else if (value === "allTime") void selectAllTime();
		else onSelect(new DateRange(value as RangeName));
	};

	return (
		<div className={styles.container}>
			<button
				type="button"
				className={cls("button-ghost", styles.stepButton)}
				aria-label="Previous date range"
				aria-keyshortcuts="ArrowLeft"
				onClick={handleSelect(range.previous())}
			>
				<ChevronLeftIcon size="24" />
			</button>
			<button
				type="button"
				className={cls("button-ghost", styles.stepButton)}
				aria-label="Next date range"
				aria-keyshortcuts="ArrowRight"
				onClick={handleSelect(range.next())}
			>
				<ChevronRightIcon size="24" />
			</button>
			<div className={styles.desktopRange}>
				<Menu.Root>
					<Menu.Trigger className={styles.selectRange}>{range.format()}</Menu.Trigger>
					<Menu.Portal>
						<Menu.Positioner className={menuStyles.positioner} align="start" sideOffset={4}>
							<Menu.Popup className={menuStyles.popup}>
								{Object.entries(wellKnownRanges).map(([key, value]) => (
									<Menu.Item
										key={key}
										className={menuStyles.item}
										data-selected={key === range.serialize() ? "true" : undefined}
										onClick={handleSelect(new DateRange(key as RangeName))}
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
							</Menu.Popup>
						</Menu.Positioner>
					</Menu.Portal>
				</Menu.Root>
			</div>
			<select
				className={styles.mobileRange}
				aria-label="Date range"
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
			</select>
			<Dialog
				className={styles.rangeDialog}
				description="Choose a start and end date for the report."
				open={customOpen}
				onOpenChange={setCustomOpen}
				title="Custom Range"
				autoOverflow
			>
				<DatePickerRange
					onSelect={(value) => {
						onSelect(value);
						setCustomOpen(false);
					}}
				/>
			</Dialog>
		</div>
	);
};
