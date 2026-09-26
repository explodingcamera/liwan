/** Options shared by the Express and Hono tracking middleware. */
export type TrackingOptions = {
	/** Defaults to pageview. */
	eventName?: string;
	/** Path prefixes to include. Empty includes match every path. */
	include?: readonly string[];
	/** Path prefixes to exclude, evaluated before includes. */
	exclude?: readonly string[];
	/** HTTP methods to track. Defaults to GET. */
	methods?: readonly string[];
	/** Receives queue or metadata errors without interrupting the request. */
	onError?: (error: unknown) => void;
};

export function shouldTrack(method: string, path: string, options: TrackingOptions): boolean {
	return (
		(options.methods ?? ["GET"]).includes(method) &&
		(!options.include?.length || options.include.some((prefix) => path.startsWith(prefix))) &&
		!options.exclude?.some((prefix) => path.startsWith(prefix))
	);
}
