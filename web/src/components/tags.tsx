import { ReactTags, type TagSelected, type TagSuggestion } from "react-tag-autocomplete";
import styles from "./tags.module.css";
import { useId } from "react";

export const Tags = ({
	onAdd,
	onDelete,
	selected,
	suggestions,
	labelText,
	noOptionsText,
}: {
	onAdd: (tag: TagSuggestion) => void;
	onDelete: (i: number) => void;
	selected: TagSelected[];
	suggestions: TagSuggestion[];
	noOptionsText: string;
	labelText?: string | React.ReactNode;
}) => {
	const id = useId();
	return (
		<>
			{labelText && (
				<label htmlFor={id} className={styles.label}>
					{labelText}
				</label>
			)}
			<ReactTags
				id={id}
				onAdd={onAdd}
				onDelete={onDelete}
				selected={selected}
				suggestions={suggestions}
				noOptionsText={noOptionsText}
				renderInput={({ classNames, inputWidth, "aria-invalid": _, ...props }) => {
					return (
						<input
							{...props}
							className={classNames.input}
							style={{ "--input-width": `${inputWidth}px` } as React.CSSProperties}
						/>
					);
				}}
				classNames={{
					root: styles["react-tags"],
					rootIsActive: styles["is-active"],
					rootIsDisabled: styles["is-disabled"],
					rootIsInvalid: styles["is-invalid"],
					label: styles["react-tags__label"],
					tagList: styles["react-tags__list"],
					tagListItem: styles["react-tags__list-item"],
					tag: styles["react-tags__tag"],
					tagName: styles["react-tags__tag-name"],
					comboBox: styles["react-tags__combobox"],
					input: styles["react-tags__combobox-input"],
					listBox: styles["react-tags__listbox"],
					option: styles["react-tags__listbox-option"],
					optionIsActive: styles["is-active"],
					highlight: styles["react-tags__listbox-option-highlight"],
				}}
			/>
		</>
	);
};
