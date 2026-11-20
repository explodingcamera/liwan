import menuStyles from "@/components/ui/menu.module.css";
import styles from "./range.module.css";

import type { ReactNode } from "react";
import { Menu } from "@base-ui/react/menu";

import { cls } from "@/utils";

/** Shared trigger and popup layout for the two date dropdowns. */
export const RangeMenu = ({
	label,
	children,
	className,
	title,
	ariaLabel,
}: {
	label: ReactNode;
	children: ReactNode;
	className?: string;
	title?: string;
	ariaLabel?: string;
}) => (
	<Menu.Root>
		<Menu.Trigger className={cls(styles.selectRange, className)} title={title} aria-label={ariaLabel}>
			{label}
		</Menu.Trigger>
		<Menu.Portal>
			<Menu.Positioner className={menuStyles.positioner} align="end" sideOffset={4}>
				<Menu.Popup className={cls(menuStyles.popup, styles.rangePopup)}>{children}</Menu.Popup>
			</Menu.Positioner>
		</Menu.Portal>
	</Menu.Root>
);
