type ClassName = string | undefined | null | false;

export const cls = (class1: ClassName | ClassName[], ...classes: (ClassName | ClassName[])[]) =>
	[class1, ...classes.flat()]
		.flat()
		.filter((cls): cls is string => typeof cls === "string" && cls.length > 0)
		.join(" ");
