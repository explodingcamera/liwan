import type { EventMetadata } from "./client";
import { type ClientIpConfig, createClientIpResolver } from "./client-ip";

export type ExpressRequestLike = {
	originalUrl: string;
	protocol: string;
	host?: string;
	ip?: string;
	get(name: string): string | undefined;
};

/** Converts trusted Express request metadata without depending on Express. */
export function fromExpressRequest(request: ExpressRequestLike): EventMetadata {
	const host = request.get("host") ?? request.host;
	if (!host) throw new Error("request host is required");
	return {
		url: `${request.protocol}://${host}${request.originalUrl}`,
		referrer: request.get("referrer") ?? request.get("referer"),
		userAgent: request.get("user-agent"),
		ip: request.ip,
	};
}

export type NodeRequestLike = {
	url?: string;
	headers: Record<string, string | string[] | undefined>;
	socket: { remoteAddress?: string };
};

export function headersFromNode(headers: NodeRequestLike["headers"]): Headers {
	const result = new Headers();
	for (const [name, value] of Object.entries(headers)) {
		if (typeof value === "string") result.set(name, value);
	}
	return result;
}

/** Resolves a Node request's client IP, using forwarding headers only for trusted peers. */
export function clientIpFromRequest(request: NodeRequestLike, config?: ClientIpConfig): string | undefined {
	return createClientIpResolver(config)(headersFromNode(request.headers), request.socket.remoteAddress);
}

export type NodeRequestOptions = {
	/** Public origin of the application, for example https://example.com. */
	origin: string;
	clientIp?: ClientIpConfig;
};

/** Captures URL, referrer, user agent, and resolved client IP from a Node HTTP request. */
export function fromNodeRequest(request: NodeRequestLike, options: NodeRequestOptions): EventMetadata {
	const referrer = request.headers.referer;
	const userAgent = request.headers["user-agent"];
	return {
		url: new URL(request.url ?? "/", options.origin).toString(),
		referrer: typeof referrer === "string" ? referrer : undefined,
		userAgent: typeof userAgent === "string" ? userAgent : undefined,
		ip: clientIpFromRequest(request, options.clientIp),
	};
}

/** Captures metadata from a standard Web Request. Pass an IP resolved by your runtime or framework. */
export function fromWebRequest(request: Request, ip?: string): EventMetadata {
	return {
		url: request.url,
		referrer: request.headers.get("referer") ?? undefined,
		userAgent: request.headers.get("user-agent") ?? undefined,
		ip,
	};
}
