import type { OASClient } from "fets";

import { appPath } from "@/config";
import type { DashboardSpec } from "@/constants";

type RequestParams = { params?: Record<string, string>; json?: unknown };

const request = async (path: string, method: string, { params, json }: RequestParams = {}) => {
	for (const [key, value] of Object.entries(params ?? {})) {
		if (value) path = path.replace(`{${key}}`, encodeURIComponent(value));
	}

	const res = await fetch(appPath(path), {
		method: method.toUpperCase(),
		credentials: "same-origin",
		headers: json ? { "Content-Type": "application/json" } : {},
		body: json ? JSON.stringify(json) : undefined,
	});

	if (!res.ok) {
		const body = await res.json().catch(() => Promise.reject({ status: res.status, message: res.statusText }));
		return Promise.reject({ status: res.status, message: body?.message ?? res.statusText });
	}
	return res;
};

// Minimal client based on fets' `createClient`, supporting only `params` and `json`.
// fets is only used for types, its runtime pulls in qs and other dependencies we don't need.
export const api = new Proxy({} as OASClient<DashboardSpec>, {
	get: (_, path: string) =>
		new Proxy(
			{},
			{
				get: (_, method: string) => (requestParams?: RequestParams) => {
					const response = request(path, method, requestParams);
					return Object.assign(response, { json: () => response.then((res) => res.json()) });
				},
			},
		),
});
