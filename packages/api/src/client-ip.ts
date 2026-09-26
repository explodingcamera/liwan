import * as ipaddr from "ipaddr.js";

export type ClientIpConfig = {
	/** Forwarding headers or provider names, checked in order. */
	sources?: readonly string[];
	/** Directly trusted proxy IPs, CIDRs, or "*". */
	trustedProxies?: readonly string[];
};

const providerHeaders: Record<string, string> = {
	akamai: "true-client-ip",
	cloudflare: "cf-connecting-ip",
	cloudfront: "cloudfront-viewer-address",
	fastly: "fastly-client-ip",
	fly: "fly-client-ip",
};

function parseIp(value: string): ipaddr.IPv4 | ipaddr.IPv6 | undefined {
	if (ipaddr.IPv4.isValidFourPartDecimal(value)) return ipaddr.IPv4.parse(value);
	if (value.includes(":") && ipaddr.IPv6.isValid(value)) {
		const address = ipaddr.IPv6.parse(value);
		if (!address.zoneId) return address;
	}
}

/** Builds a reusable IP resolver. Forwarding headers are ignored unless the direct peer is trusted. */
export function createClientIpResolver(
	config: ClientIpConfig = {},
): (headers: Headers, peer?: string) => string | undefined {
	const sources = (config.sources ?? []).map((source) => {
		const name = source.trim().toLowerCase().replaceAll("_", "-");
		return providerHeaders[name] ?? name;
	});
	const proxies = (config.trustedProxies ?? []).map((value) => {
		if (value === "*") return () => true;
		if (value.includes("/")) {
			const [network, bits] = ipaddr.parseCIDR(value);
			if (!parseIp(value.split("/", 1)[0])) throw new Error("invalid trusted proxy");
			return (ip: ipaddr.IPv4 | ipaddr.IPv6) => ip.kind() === network.kind() && ip.match(network, bits);
		}
		const proxy = parseIp(value);
		if (!proxy) throw new Error("invalid trusted proxy");
		return (ip: ipaddr.IPv4 | ipaddr.IPv6) => ip.kind() === proxy.kind() && ip.toString() === proxy.toString();
	});
	const trusted = (ip: ipaddr.IPv4 | ipaddr.IPv6) => proxies.some((proxy) => proxy(ip));

	return (headers, peer) => {
		const peerIp = peer && parseIp(peer);
		if (!peerIp) return undefined;
		if (!trusted(peerIp)) return peer;

		for (const source of sources) {
			const value = headers.get(source)?.trim();
			if (!value) continue;
			if (source === "x-forwarded-for" || source === "forwarded") {
				const chain = value.split(",").map((part) => {
					if (source === "x-forwarded-for") return part.trim();
					return part
						.split(";")
						.map((entry) => entry.trim())
						.find((entry) => entry.startsWith("for="))
						?.slice(4)
						.replace(/^"|"$/g, "");
				});
				const addresses = chain.map((address) => address && parseIp(address));
				if (addresses.some((address) => !address)) continue;
				let current = peerIp;
				let result = peer;
				for (let index = chain.length - 1; index >= 0; index--) {
					if (!trusted(current)) break;
					const address = addresses[index];
					if (!address) break;
					current = address;
					result = chain[index] ?? result;
				}
				return result;
			}
			if (source === "cloudfront-viewer-address") {
				const socket = /^(?:\[([^\]]+)\]|(\d+(?:\.\d+){3})):(\d+)$/.exec(value);
				if (!socket || Number(socket[3]) > 65535) continue;
				const address = socket[1] ?? socket[2];
				if (parseIp(address)) return address;
			} else if (parseIp(value)) {
				return value;
			}
		}
		return peer;
	};
}
