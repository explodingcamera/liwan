import styles from "./range.module.css";
import { useRef } from "react";
import { rangeNames, type RangeName } from "../../api/ranges";
import { cls } from "../../utils";

export const SelectRange = ({ onSelect, range }: { onSelect: (name: RangeName) => void; range: RangeName }) => {
	const detailsRef = useRef<HTMLDetailsElement>(null);

	const handleSelect = (name: RangeName) => () => {
		if (detailsRef.current) detailsRef.current.open = false;
		onSelect(name);
	};

	return (
		<details ref={detailsRef} className={cls("dropdown", styles.selectRange)}>
			<summary>{rangeNames[range]}</summary>
			<ul>
				{Object.entries(rangeNames).map(([key, value]) => (
					<li key={key}>
						{/* biome-ignore lint/a11y/useValidAnchor: this is fine */}
						<a className={key === range ? styles.selected : ""} onClick={handleSelect(key as RangeName)}>
							{value}
						</a>
					</li>
				))}
			</ul>
		</details>
	);
};
