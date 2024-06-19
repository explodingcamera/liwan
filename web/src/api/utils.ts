import {
	QueryClient,
	useQuery as _useQuery,
	useMutation as _useMutation,
	type DefaultError,
	type QueryKey,
	type UseQueryOptions,
	type UseQueryResult,
} from "@tanstack/react-query";

export const queryClient = new QueryClient();

export const useMutation: typeof _useMutation = (options, c) => _useMutation(options, c || queryClient);

export function useQuery<
	TQueryFnData = unknown,
	TError = DefaultError,
	TData = TQueryFnData,
	TQueryKey extends QueryKey = QueryKey,
>(options: UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>, c?: QueryClient): UseQueryResult<TData, TError> {
	return _useQuery(options, c || queryClient);
}

type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH";

export const req = <T>(method: HttpMethod, url: string, body?: unknown): Promise<T> => {
	return fetch(url, {
		method,
		credentials: "same-origin",
		headers: {
			"Content-Type": "application/json",
		},
		body: body ? JSON.stringify(body) : undefined,
	})
		.then((response) => {
			if (!response.ok)
				return response.json().then((errorData) => Promise.reject(errorData?.message || response.statusText));
			return response.json();
		})
		.then((data) => data?.data as T)
		.catch((error) => Promise.reject(error));
};

// get the username cookie or undefined if not set
export const getUsername = () => {
	const username = document.cookie.match(/username=(.*?)(;|$)/);
	return username ? username[1] : undefined;
};
