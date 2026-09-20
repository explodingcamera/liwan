import styles from "./font-size-switcher.module.css";

import { Check, Minus, Plus } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cls } from "@/utils";

export type FontSize = "compact" | "small" | "normal" | "large";

export interface FontSizeOption {
	id: FontSize;
	label: string;
	scale: string;
}

export const FONT_SIZES: FontSizeOption[] = [
	{ id: "compact", label: "Compact", scale: "75%" },
	{ id: "small", label: "Small", scale: "85%" },
	{ id: "normal", label: "Normal", scale: "100%" },
	{ id: "large", label: "Large", scale: "115%" },
];

export const getStoredFontSize = (): FontSize => {
	if (typeof window === "undefined") return "normal";
	const currentAttr = document.documentElement.getAttribute("data-font-size") as FontSize | null;
	if (currentAttr && FONT_SIZES.some((s) => s.id === currentAttr)) {
		return currentAttr;
	}
	const stored = window.localStorage?.getItem("liwan-font-size") as FontSize | null;
	if (stored && FONT_SIZES.some((s) => s.id === stored)) {
		return stored;
	}
	return "normal";
};

export const setAppFontSize = (size: FontSize) => {
	if (typeof document === "undefined") return;
	document.documentElement.setAttribute("data-font-size", size);
	try {
		window.localStorage?.setItem("liwan-font-size", size);
	} catch {}

	window.dispatchEvent(new CustomEvent("liwan:font-size", { detail: size }));
	requestAnimationFrame(() => {
		window.dispatchEvent(new Event("resize"));
	});
};

export const FontSizeSwitcher = () => {
	const [activeSize, setActiveSize] = useState<FontSize>("normal");
	const detailsRef = useRef<HTMLDetailsElement>(null);

	useEffect(() => {
		const current = getStoredFontSize();
		setActiveSize(current);
		if (typeof document !== "undefined") {
			document.documentElement.setAttribute("data-font-size", current);
		}

		const handleCustomChange = (e: Event) => {
			const customEvent = e as CustomEvent<FontSize>;
			if (customEvent.detail) {
				setActiveSize(customEvent.detail);
			}
		};

		const handleClickOutside = (e: MouseEvent) => {
			if (detailsRef.current && detailsRef.current.open && !detailsRef.current.contains(e.target as Node)) {
				detailsRef.current.open = false;
			}
		};

		window.addEventListener("liwan:font-size", handleCustomChange);
		document.addEventListener("click", handleClickOutside);
		return () => {
			window.removeEventListener("liwan:font-size", handleCustomChange);
			document.removeEventListener("click", handleClickOutside);
		};
	}, []);

	const currentIndex = FONT_SIZES.findIndex((s) => s.id === activeSize);
	const activeConfig = FONT_SIZES[currentIndex >= 0 ? currentIndex : 2];

	const selectSize = useCallback((size: FontSize) => {
		setActiveSize(size);
		setAppFontSize(size);
		if (detailsRef.current) {
			detailsRef.current.open = false;
		}
	}, []);

	const handleStepDown = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation();
			const idx = FONT_SIZES.findIndex((s) => s.id === activeSize);
			if (idx > 0) {
				const nextSize = FONT_SIZES[idx - 1].id;
				setActiveSize(nextSize);
				setAppFontSize(nextSize);
			}
		},
		[activeSize],
	);

	const handleStepUp = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation();
			const idx = FONT_SIZES.findIndex((s) => s.id === activeSize);
			if (idx < FONT_SIZES.length - 1) {
				const nextSize = FONT_SIZES[idx + 1].id;
				setActiveSize(nextSize);
				setAppFontSize(nextSize);
			}
		},
		[activeSize],
	);

	return (
		<details ref={detailsRef} className={cls("dropdown", "right", styles.fontSizeDropdown)}>
			<summary
				role="button"
				className={cls("outline", "secondary", styles.summary)}
				aria-label="Font size settings"
				title={`Font size: ${activeConfig.label} (${activeConfig.scale})`}
			>
				<span className={styles.aaText}>Aa</span>
			</summary>
			<ul className={styles.dropdownMenu}>
				<li className={styles.menuHeader}>
					<span>Font Size</span>
					<span className={styles.badge}>{activeConfig.scale}</span>
				</li>
				<li className={styles.stepperItem}>
					<div className={styles.stepper}>
						<button
							type="button"
							className={styles.stepBtn}
							onClick={handleStepDown}
							disabled={currentIndex <= 0}
							title="Decrease font size (A-)"
						>
							<Minus size={13} />
							<span>A-</span>
						</button>
						<span className={styles.stepperCurrent}>{activeConfig.label}</span>
						<button
							type="button"
							className={styles.stepBtn}
							onClick={handleStepUp}
							disabled={currentIndex >= FONT_SIZES.length - 1}
							title="Increase font size (A+)"
						>
							<span>A+</span>
							<Plus size={13} />
						</button>
					</div>
				</li>
				<li className={styles.divider}>
					<hr />
				</li>
				{FONT_SIZES.map((item) => {
					const isSelected = activeSize === item.id;
					return (
						<li key={item.id} className={styles.optionItem}>
							<button
								type="button"
								className={cls(styles.optionBtn, isSelected && styles.activeOption)}
								onClick={() => selectSize(item.id)}
							>
								<span className={styles.optionLabel}>{item.label}</span>
								<span className={styles.optionScale}>{item.scale}</span>
								{isSelected && <Check size={14} className={styles.checkIcon} />}
							</button>
						</li>
					);
				})}
			</ul>
		</details>
	);
};
