import type { RequestHandler } from "express";

import type { Client } from "./client";
import { type ClientIpConfig, createClientIpResolver } from "./client-ip";
import type { TrackingOptions } from "./middleware";
import { shouldTrack } from "./middleware";
import { fromExpressRequest, headersFromNode } from "./request";

export type ExpressTrackingOptions = TrackingOptions & {
	/** Public origin to use instead of the incoming Host header. */
	origin?: string;
	/** Resolve forwarding headers from explicitly trusted proxies instead of using Express's request.ip. */
	clientIp?: ClientIpConfig;
};

/** Records matching Express requests without delaying the response. */
export function expressMiddleware(
	client: Client,
	entityId: string,
	options: ExpressTrackingOptions = {},
): RequestHandler {
	const resolveIp = options.clientIp && createClientIpResolver(options.clientIp);
	return (request, _response, next) => {
		if (shouldTrack(request.method, request.path, options)) {
			try {
				const metadata = fromExpressRequest(request);
				if (options.origin) metadata.url = new URL(request.originalUrl, options.origin).toString();
				if (resolveIp) metadata.ip = resolveIp(headersFromNode(request.headers), request.socket.remoteAddress);
				client.event(entityId, options.eventName ?? "pageview", metadata);
			} catch (error) {
				options.onError?.(error);
			}
		}
		next();
	};
}
