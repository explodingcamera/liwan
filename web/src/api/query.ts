import {
	type DefaultError,
	QueryClient,
	type QueryKey,
	type UseQueryOptions,
	type UseQueryResult,
	useMutation as _useMutation,
	useQuery as _useQuery,
} from "@tanstack/react-query";

export const queryClient = new QueryClient();
export const useMutation: typeof _useMutation = (options, c) => _useMutation(options, c || queryClient);

export function useQuery<
	TQueryFnData = unknown,
	TError = DefaultError,
	TData = TQueryFnData,
	TQueryKey extends QueryKey = QueryKey,
>(options: UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>, c?: QueryClient): UseQueryResult<TData, TError> {
	return _useQuery(
		{
			enabled(query) {
				if (typeof window === "undefined") return false;
				return typeof options.enabled === "function" ? options.enabled(query) : (options.enabled ?? true);
			},
			...options,
		},
		c || queryClient,
	);
}
