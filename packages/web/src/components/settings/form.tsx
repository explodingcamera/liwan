import styles from "./form.module.css";

import { cloneElement, isValidElement, type ReactNode, type SubmitEvent, useId } from "react";
import { Field } from "@base-ui/react/field";
import { Fieldset } from "@base-ui/react/fieldset";
import { Form as BaseForm } from "@base-ui/react/form";
import { Switch } from "@base-ui/react/switch";
import { Tabs } from "@base-ui/react/tabs";
import { ArrowLeftIcon, SettingsIcon } from "lucide-react";

import { basePath } from "@/config";
import { cls } from "@/utils";

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
	backHref,
	backLabel,
	saveForm,
}: {
	title: ReactNode;
	backHref?: string;
	backLabel?: string;
	saveForm?: string;
}) => (
	<nav className={styles.header} data-has-back={backHref ? true : undefined}>
		<div className={styles.titleGroup}>
			{backHref && (
				<a href={backHref} className={styles.backButton} aria-label={backLabel ?? "Back"}>
					<ArrowLeftIcon size={20} />
				</a>
			)}
			<h1>{title}</h1>
		</div>
		{saveForm && (
			<button type="submit" form={saveForm} className={`${styles.saveButton} button-primary`}>
				Save
			</button>
		)}
	</nav>
);

/** Icon link to a settings detail page, used in settings tables. */
export const SettingsLink = ({ href, label }: { href: string; label: string }) => (
	<a href={href} className={styles.settingsLink} aria-label={label} title={label}>
		<SettingsIcon size={18} />
	</a>
);

/** Returns the part of the current path after `prefix`, or an empty string if the path doesn't match. */
export const getSettingsPathId = (prefix: string) => {
	const path = window.location.pathname.slice(basePath.length).replace(/\/$/, "");
	return path.startsWith(prefix) ? path.slice(prefix.length) : "";
};

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

export const SettingsPanel = ({
	value,
	className,
	children,
}: {
	value: string;
	className?: string;
	children: ReactNode;
}) => (
	<Tabs.Panel value={value} className={cls(styles.panel, className)}>
		{children}
	</Tabs.Panel>
);

export const SettingsField = ({
	label,
	description,
	name,
	htmlFor,
	className,
	disabled,
	children,
}: {
	label?: ReactNode;
	description?: ReactNode;
	name?: string;
	htmlFor?: string;
	className?: string;
	disabled?: boolean;
	children: ReactNode;
}) => {
	const id = useId();
	let control = children;
	let labelFor = htmlFor;
	if (isValidElement<{ id?: string }>(children) && ["input", "select", "textarea"].includes(String(children.type))) {
		labelFor ??= children.props.id ?? id;
		control = cloneElement(children, { id: children.props.id ?? labelFor });
	}
	return (
		<Field.Root name={name} className={cls(styles.field, className)} data-disabled={disabled ? true : undefined}>
			{label && (
				<Field.Label htmlFor={labelFor} className={styles.label}>
					{label}
				</Field.Label>
			)}
			{description && <Field.Description className={styles.fieldDescription}>{description}</Field.Description>}
			{control}
		</Field.Root>
	);
};

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
