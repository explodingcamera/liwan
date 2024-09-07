import * as Dia from "@radix-ui/react-dialog";
import { XIcon } from "lucide-react";

import { cls } from "../utils";
import styles from "./dialog.module.css";

export type DialogProps = {
	title: string;
	description?: string;
	hideDescription?: boolean;
	trigger: React.ReactNode | (() => React.ReactNode);
	children: React.ReactNode;
	onOpenChange?: (open: boolean) => void;
	className?: string;
	showClose?: boolean;
	hideTitle?: boolean;
	autoOverflow?: boolean;
};

export const Dialog = ({
	title,
	description,
	hideDescription,
	trigger,
	children,
	onOpenChange,
	className,
	showClose,
	hideTitle,
	autoOverflow,
}: DialogProps) => {
	return (
		<Dia.Root onOpenChange={onOpenChange}>
			<Dia.Trigger asChild>{typeof trigger === "function" ? trigger() : trigger}</Dia.Trigger>
			<Dia.Portal>
				<Dia.Overlay className={styles.overlay} />
				{showClose && (
					<Dia.Close asChild>
						<button type="button" className={styles.close}>
							<XIcon size="24" />
						</button>
					</Dia.Close>
				)}

				<Dia.Content asChild>
					<article className={cls(styles.content, className, autoOverflow && styles.autoOverflow)}>
						<Dia.Title className={styles.title} hidden={hideTitle}>
							{title}
						</Dia.Title>
						{description && (
							<Dia.Description hidden={hideDescription} className={styles.description}>
								{description}
							</Dia.Description>
						)}
						{children}
					</article>
				</Dia.Content>
			</Dia.Portal>
		</Dia.Root>
	);
};

Dialog.Close = Dia.Close;
