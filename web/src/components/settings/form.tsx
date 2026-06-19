import styles from "./form.module.css";

import type { ReactNode, SubmitEvent } from "react";
import { Field } from "@base-ui/react/field";
import { Fieldset } from "@base-ui/react/fieldset";
import { Form as BaseForm } from "@base-ui/react/form";
import { Switch } from "@base-ui/react/switch";
import { Tabs } from "@base-ui/react/tabs";
import { ArrowLeftIcon } from "lucide-react";

type TabItem<T extends string> = { value: T; label: ReactNode };

export const SettingsForm = ({
	id,
	onSubmit,
	children,
}: {
	id?: string;
	onSubmit?: (event: SubmitEvent<HTMLFormElement>) => void;
	children: ReactNode;
}) => (
	<BaseForm id={id} className={styles.form} onSubmit={onSubmit ?? ((event) => event.preventDefault())}>
		{children}
	</BaseForm>
);

export const SettingsHeader = ({
	title,
	description,
	backHref,
	backLabel,
	saveForm,
}: {
	title: ReactNode;
	description?: ReactNode;
	backHref?: string;
	backLabel?: string;
	saveForm?: string;
}) => (
	<>
		<nav className={styles.header}>
			<div className={styles.titleGroup}>
				{backHref && (
					<a href={backHref} className={styles.backButton} aria-label={backLabel ?? "Back"}>
						<ArrowLeftIcon size={20} />
					</a>
				)}
				<h1>{title}</h1>
			</div>
			{saveForm && (
				<button type="submit" form={saveForm} className={styles.saveButton}>
					Save
				</button>
			)}
		</nav>
		{description && <p className={styles.description}>{description}</p>}
	</>
);

export const SettingsTabs = <T extends string>({
	value,
	onValueChange,
	tabs,
	children,
}: {
	value: T;
	onValueChange: (value: T) => void;
	tabs: readonly TabItem<T>[];
	children: ReactNode;
}) => (
	<Tabs.Root value={value} onValueChange={(next) => onValueChange(next as T)}>
		<Tabs.List className={styles.tabs}>
			{tabs.map((tab) => (
				<Tabs.Tab key={tab.value} value={tab.value} className={styles.tab}>
					{tab.label}
				</Tabs.Tab>
			))}
		</Tabs.List>
		{children}
	</Tabs.Root>
);

export const SettingsPanel = ({ value, children }: { value: string; children: ReactNode }) => (
	<Tabs.Panel value={value} className={styles.panel}>
		{children}
	</Tabs.Panel>
);

export const SettingsField = ({
	label,
	description,
	name,
	children,
}: {
	label: ReactNode;
	description?: ReactNode;
	name?: string;
	children: ReactNode;
}) => (
	<Field.Root name={name} className={styles.field}>
		<Field.Label className={styles.label}>{label}</Field.Label>
		{description && <Field.Description className={styles.fieldDescription}>{description}</Field.Description>}
		{children}
	</Field.Root>
);

export const SettingsFieldset = ({
	legend,
	description,
	children,
}: {
	legend: ReactNode;
	description?: ReactNode;
	children: ReactNode;
}) => (
	<Fieldset.Root className={styles.fieldset}>
		<Fieldset.Legend className={styles.legend}>{legend}</Fieldset.Legend>
		{description && <p className={styles.fieldsetDescription}>{description}</p>}
		{children}
	</Fieldset.Root>
);

export const SettingsSwitch = ({
	label,
	description,
	name,
	checked,
	disabled,
	onCheckedChange,
}: {
	label: ReactNode;
	description?: ReactNode;
	name?: string;
	checked: boolean;
	disabled?: boolean;
	onCheckedChange: (checked: boolean) => void;
}) => (
	<Field.Root name={name} className={styles.switchField}>
		<div className={styles.switchText}>
			<Field.Label className={styles.label}>{label}</Field.Label>
			{description && <Field.Description className={styles.fieldDescription}>{description}</Field.Description>}
		</div>
		<Switch.Root
			name={name}
			checked={checked}
			disabled={disabled}
			onCheckedChange={onCheckedChange}
			className={styles.switchRoot}
		>
			<Switch.Thumb className={styles.switchThumb} />
		</Switch.Root>
	</Field.Root>
);
