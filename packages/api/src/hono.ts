import type { Context, MiddlewareHandler } from "hono";

import type { Client } from "./client";
import { type ClientIpConfig, createClientIpResolver } from "./client-ip";
import type { TrackingOptions } from "./middleware";
import { shouldTrack } from "./middleware";
import { fromWebRequest } from "./request";

export type HonoTrackingOptions = TrackingOptions & {
	/** Resolve the direct peer using the hosting runtime's connection information. */
	peerIp?: (context: Context) => string | undefined;
	/** Forwarding headers and trusted proxies, like the Rust client. */
	clientIp?: ClientIpConfig;
};

/** Records matching Hono requests without delaying the response. */
export function honoMiddleware(client: Client, entityId: string, options: HonoTrackingOptions = {}): MiddlewareHandler {
	const resolveIp = createClientIpResolver(options.clientIp);
	return async (context, next) => {
		if (shouldTrack(context.req.method, context.req.path, options)) {
			try {
				client.event(
					entityId,
					options.eventName ?? "pageview",
					fromWebRequest(context.req.raw, resolveIp(context.req.raw.headers, options.peerIp?.(context))),
				);
			} catch (error) {
				options.onError?.(error);
			}
		}
		await next();
	};
}
