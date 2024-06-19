import { fetchGroups, useQuery } from "../api";

type Group = {
	id: string;
	displayName: string;
	entities: Record<string, string>;
};

const dummyGroups: Group[] = [
	{
		id: "personal",
		displayName: "Personal Websites",
		entities: {
			portfolio: "Portfolio",
			blog: "Blog",
		},
	},
	{
		id: "dawdle.space",
		displayName: "Dawdle Space",
		entities: {
			"dawdle.space": "Dawdle Space",
			"lastfm-iceberg": "Lastfm Iceberg",
		},
	},
];

export const Groups = () => {
	const { data, isLoading } = useQuery({
		queryKey: ["groups"],
		queryFn: fetchGroups,
	});
	const groups = dummyGroups;

	return (
		<div>
			{groups.map((group) => (
				<article key={group.id}>
					<h4>{group.displayName}</h4>
					<ul>
						{Object.entries(group.entities).map(([id, displayName]) => (
							<li key={id}>{displayName}</li>
						))}
					</ul>
				</article>
			))}
		</div>
	);
};
