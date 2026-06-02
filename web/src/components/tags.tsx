import styles from "./tags.module.css";

import { Fragment, useId } from "react";
import { Combobox } from "@base-ui/react/combobox";
import { CheckIcon, XIcon } from "lucide-react";

export type Tag = {
	value: string;
	label: string;
};

export const Tags = ({
	onAdd,
	onDelete,
	selected,
	suggestions,
	labelText,
	placeholderText,
	noOptionsText,
}: {
	onAdd: (tag: Tag) => void;
	onDelete: (i: number) => void;
	selected: Tag[];
	suggestions: Tag[];
	noOptionsText: string;
	placeholderText?: string;
	labelText?: string | React.ReactNode;
}) => {
	const id = useId();
	const items = suggestions.filter((suggestion) => !selected.some((tag) => tag.value === suggestion.value));

	const handleValueChange = (next: Tag[]) => {
		const added = next.find((tag) => !selected.some((selectedTag) => selectedTag.value === tag.value));
		if (added) {
			onAdd(added);
			return;
		}

		const removedIndex = selected.findIndex((tag) => !next.some((nextTag) => nextTag.value === tag.value));
		if (removedIndex >= 0) onDelete(removedIndex);
	};

	return (
		<Combobox.Root
			items={items}
			value={selected}
			onValueChange={handleValueChange}
			itemToStringLabel={(item) => item.label}
			itemToStringValue={(item) => item.value}
			isItemEqualToValue={(item, value) => item.value === value.value}
			multiple
		>
			<div className={styles.container}>
				{labelText && (
					<label htmlFor={id} className={styles.label}>
						{labelText}
					</label>
				)}
				<Combobox.InputGroup className={styles.inputGroup}>
					<Combobox.Chips className={styles.chips}>
						<Combobox.Value>
							{(value: Tag[]) => (
								<Fragment>
									{value.map((tag) => (
										<Combobox.Chip key={tag.value} className={styles.chip} aria-label={tag.label}>
											{tag.label}
											<Combobox.ChipRemove className={styles.chipRemove} aria-label={`Remove ${tag.label}`}>
												<XIcon size={14} />
											</Combobox.ChipRemove>
										</Combobox.Chip>
									))}
									<Combobox.Input
										id={id}
										placeholder={value.length > 0 ? "" : (placeholderText ?? "Type to search...")}
										className={styles.input}
									/>
								</Fragment>
							)}
						</Combobox.Value>
					</Combobox.Chips>
				</Combobox.InputGroup>
			</div>

			<Combobox.Portal>
				<Combobox.Positioner className={styles.positioner} sideOffset={4}>
					<Combobox.Popup className={styles.popup}>
						<Combobox.Empty>
							<div className={styles.empty}>{noOptionsText ?? "No matching options..."}</div>
						</Combobox.Empty>
						<Combobox.List className={styles.list}>
							{(tag: Tag) => (
								<Combobox.Item key={tag.value} value={tag} className={styles.item}>
									<Combobox.ItemIndicator className={styles.itemIndicator}>
										<CheckIcon size={14} />
									</Combobox.ItemIndicator>
									<span className={styles.itemText}>{tag.label}</span>
								</Combobox.Item>
							)}
						</Combobox.List>
					</Combobox.Popup>
				</Combobox.Positioner>
			</Combobox.Portal>
		</Combobox.Root>
	);
};
