import type { GeometryCollection, Topology, Transform } from "topojson-specification";

export type MapTopology = Topology<{
	geo: GeometryCollection<{ name: string; iso: string }>;
}> & { transform: Transform };

export const createMapFeatures = (geo: MapTopology) => {
	const { scale, translate } = geo.transform;
	const arcs = geo.arcs.map((arc) => {
		let x = 0;
		let y = 0;
		return arc.map(([dx, dy]) => {
			x += dx;
			y += dy;
			return `${(x * scale[0] + translate[0]).toFixed(1)},${(y * scale[1] + translate[1]).toFixed(1)}`;
		});
	});
	const ringPath = (ring: number[]) =>
		`M${ring.flatMap((index) => (index < 0 ? [...arcs[~index]].reverse() : arcs[index])).join("L")}Z`;

	return geo.objects.geo.geometries.flatMap((geometry) => {
		if (geometry.type !== "Polygon" && geometry.type !== "MultiPolygon") return [];
		const polygons = geometry.type === "Polygon" ? [geometry.arcs] : geometry.arcs;
		return [
			{
				path: polygons.flat().map(ringPath).join(""),
				name: geometry.properties?.name ?? "",
				iso: geometry.properties?.iso ?? "",
			},
		];
	});
};
