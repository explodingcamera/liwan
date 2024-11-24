import { cls } from "../utils";
import styles from "./card.module.css";

export const CardButton = ({
	active,
	children,
	onClick,
	className,
	type = "button",
	disabled,
}: {
	children?: React.ReactNode;
	active?: boolean;
	onClick?: () => void;
	className?: string;
	type?: "button" | "submit" | "reset";
	disabled?: boolean;
}) => {
	return (
		<button
			type={type}
			className={cls(className, styles.card)}
			onClick={onClick}
			disabled={disabled}
			data-active={active}
		>
			{children}
		</button>
	);
};

export const CardLink = ({
	href,
	target,
	children,
	className,
}: {
	href: string;
	target?: string;
	children?: React.ReactNode;
	className?: string;
}) => {
	return (
		<a href={href} target={target} className={cls(className, styles.card)}>
			{children}
		</a>
	);
};
