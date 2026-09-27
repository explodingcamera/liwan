import { createClient } from "fets";

import { appPath } from "@/config";
import type { DashboardSpec } from "@/constants";

export const api = createClient<DashboardSpec>({
	globalParams: { credentials: "same-origin" },
	fetchFn(input, init) {
		return fetch(typeof input === "string" && input.startsWith("/") ? appPath(input) : input, init).then((res) => {
			if (!res.ok) {
				return res
					.json()
					.catch((_) => Promise.reject({ status: res.status, message: res.statusText }))
					.then((body) =>
						Promise.reject({
							status: res.status,
							message: body?.message ?? res.statusText,
						}),
					);
			}
			return res;
		});
	},
});
