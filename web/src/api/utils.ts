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

// get the username cookie or undefined if not set
export const getUsername = () => {
	const username = document.cookie.match(/username=(.*?)(;|$)/);
	return username ? username[1] : undefined;
};

type StripPrefix<T, P extends string> = {
	[K in keyof T as K extends `${P}${infer R}` ? R : K]: T[K];
};

// biome-ignore lint/suspicious/noExplicitAny: any is needed to support arbitrary objects
export const stripPrefix = <T extends Record<string, any>, P extends string>(obj: T, prefix: P): StripPrefix<T, P> => {
	// biome-ignore lint/suspicious/noExplicitAny: any is needed to support arbitrary objects
	const result: any = {};

	for (const key in obj) {
		if (key.startsWith(prefix)) {
			const newKey = key.slice(prefix.length);
			result[newKey] = obj[key];
		} else {
			result[key] = obj[key];
		}
	}

	return result as StripPrefix<T, P>;
};
