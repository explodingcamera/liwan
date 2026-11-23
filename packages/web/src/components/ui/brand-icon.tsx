import type { ComponentPropsWithoutRef } from "react";
import type { IconDefinition } from "@fortawesome/free-brands-svg-icons";

type BrandIconProps = Omit<ComponentPropsWithoutRef<"svg">, "children"> & {
	icon: IconDefinition;
	size?: number;
};

export const BrandIcon = ({ icon, size = 24, ...props }: BrandIconProps) => {
	const [width, height, , , pathData] = icon.icon;
	const paths = typeof pathData === "string" ? [pathData] : pathData;

	return (
		<svg
			width={size}
			height={size}
			viewBox={`0 0 ${width} ${height}`}
			fill="currentColor"
			aria-hidden="true"
			{...props}
		>
			{paths.map((path) => (
				<path key={path} d={path} />
			))}
		</svg>
	);
};
