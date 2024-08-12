import styles from "./dialog.module.css";
import * as Dia from "@radix-ui/react-dialog";

export const Dialog = ({
	title,
	description,
	hideDescription,
	trigger,
	children,
	onOpenChange,
}: {
	title?: string;
	description?: string;
	hideDescription?: boolean;
	trigger: React.ReactNode;
	children: React.ReactNode;
	onOpenChange?: (open: boolean) => void;
}) => {
	return (
		<Dia.Root onOpenChange={onOpenChange}>
			<Dia.Trigger asChild>{trigger}</Dia.Trigger>
			<Dia.Portal>
				<Dia.Overlay className={styles.overlay} />

				<Dia.Content asChild>
					<article className={styles.content}>
						{title && <Dia.Title className={styles.title}>{title}</Dia.Title>}
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
