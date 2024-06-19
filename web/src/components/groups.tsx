import { fetchGroups, useQuery } from "../api";

export const Groups = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["groups"],
		queryFn: fetchGroups,
	});

	return (
		<div>
			{data &&
				Object.entries(data).map(([key, value]) => {
					return (
						<article key={key}>
							<h4>{value.displayName}</h4>
							<ul>
								{Object.entries(value.entities).map(([id, displayName]) => {
									return <li key={id}>{displayName}</li>;
								})}
							</ul>
						</article>
					);
				})}
		</div>
	);
};
