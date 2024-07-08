import styles from "./dialog.module.css";
import * as Dia from "@radix-ui/react-dialog";

export const Dialog = ({
	title,
	description,
	trigger,
	children,
}: {
	title?: string;
	description?: string;
	trigger: React.ReactNode;
	children: React.ReactNode;
}) => {
	return (
		<Dia.Root>
			<Dia.Trigger asChild>{trigger}</Dia.Trigger>
			<Dia.Portal>
				<Dia.Overlay className={styles.overlay} />

				<Dia.Content
					asChild
					onSubmit={(e) => {
						console.log("submit");
					}}
				>
					<article className={styles.content}>
						{title && <Dia.Title className={styles.title}>{title}</Dia.Title>}
						{description && <Dia.Description className={styles.description}>{description}</Dia.Description>}
						{children}
					</article>
				</Dia.Content>
			</Dia.Portal>
		</Dia.Root>
	);
};

Dialog.Close = Dia.Close;
