import { Dialog as Dia } from "@base-ui/react";
import { XIcon } from "lucide-react";

import { cls } from "../utils";
import styles from "./dialog.module.css";
import type { ReactElement } from "react";

export type DialogProps = {
	title: string;
	description?: string;
	hideDescription?: boolean;
	trigger?: ReactElement | false;
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
			{trigger && <Dia.Trigger nativeButton={trigger.type === "button"} render={trigger} />}
			<Dia.Portal>
				<Dia.Backdrop className={styles.overlay} />

				<Dia.Viewport>
					{showClose && (
						<Dia.Close className={styles.close}>
							<XIcon color="black" size="24" />
						</Dia.Close>
					)}
					<Dia.Popup>
						<article className={cls(styles.content, autoOverflow && styles.autoOverflow, className)}>
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
					</Dia.Popup>
				</Dia.Viewport>
			</Dia.Portal>
		</Dia.Root>
	);
};

Dialog.Close = Dia.Close;
