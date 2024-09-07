import { createClient } from "fets";
import type { DashboardSpec } from "./types";

export * from "./query";
export * from "./constants";
export * from "./types";
export * from "./hooks";

export const api = createClient<DashboardSpec>({
	globalParams: { credentials: "same-origin" },
	fetchFn(input, init) {
		return fetch(input, init).then((res) => {
			if (!res.ok) {
				return res
					.json()
					.catch((_) => Promise.reject({ status: res.status, message: res.statusText }))
					.then((body) => Promise.reject({ status: res.status, message: body?.message ?? res.statusText }));
			}
			return res;
		});
	},
});
