import styles from "./map.module.css";

import { RotateCcwIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Tooltip } from "react-tooltip";

import { geoMercator, geoPath } from "d3-geo";
import { select } from "d3-selection";
import { type ZoomBehavior, zoom as d3Zoom, zoomIdentity } from "d3-zoom";

import * as topo from "topojson-client";
import type { GeometryCollection, Topology } from "topojson-specification";

import geo from "../../../../data/geo.json";
import { type DimensionTableRow, type Metric, metricNames } from "../../api";
import { cls, formatMetricVal } from "../../utils";

const projection = geoMercator().scale(125).center([45, 45]);
const pathGenerator = geoPath(projection);

type GeoProps = { name: string; iso: string };

const features = topo
	.feature<GeoProps>(geo as unknown as Topology, geo.objects.geo as GeometryCollection<GeoProps>)
	.features.map((feature) => ({
		path: pathGenerator(feature),
		name: feature.properties.name,
		iso: feature.properties.iso,
	}));

type Location = {
	name: string;
	iso: string;
};

const getCountries = (data: DimensionTableRow[]) => {
	const countries = new Map<string, number>();
	for (const row of data) {
		countries.set(row.dimensionValue, row.value);
	}
	return countries;
};

export const Worldmap = ({
	metric,
	data,
}: {
	metric: Metric;
	data?: DimensionTableRow[];
}) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const containerRef = useRef<HTMLDivElement | null>(null);

	const [moved, setMoved] = useState(false);
	const [currentLocation, setCurrentLocation] = useState<Location | null>(null);

	const biggest = useMemo(() => data?.reduce((a, b) => (a.value > b.value ? a : b), data[0]), [data]);
	const countries = useMemo(() => getCountries(data ?? []), [data]);

	const zoomBehavior = useRef<ZoomBehavior<SVGSVGElement, unknown>>();
	if (!zoomBehavior.current) {
		zoomBehavior.current = d3Zoom<SVGSVGElement, unknown>()
			.scaleExtent([1, 8]) // Min and max zoom levels
			.on("zoom", (event) => {
				select(svgRef.current).select("g").attr("transform", event.transform);
				if (!moved) setMoved(true);
			});
	}

	useEffect(() => {
		if (!svgRef.current || !zoomBehavior.current) return;
		select(svgRef.current).call(zoomBehavior.current);
	}, []);

	const landmasses = useMemo(
		() =>
			features.map((feature, index) => (
				<Landmass
					key={index}
					feature={feature}
					countries={countries}
					biggest={biggest}
					onSetLocation={setCurrentLocation}
				/>
			)),
		[countries, biggest],
	);

	const resetZoom = () => {
		setCurrentLocation(null);
		if (zoomBehavior.current && svgRef.current) {
			select(svgRef.current).call(zoomBehavior.current.transform, zoomIdentity);
			setMoved(false);
		}
	};

	const currentTooltip = useMemo(
		() =>
			currentLocation && {
				name: currentLocation?.name ?? "",
				value: formatMetricVal(countries.get(currentLocation.iso) ?? 0, metric),
			},
		[currentLocation, countries, metric],
	);

	return (
		<div ref={containerRef} className={styles.worldmap} data-tooltip-float={true} data-tooltip-id="map">
			<button type="button" className={cls(styles.reset, moved && styles.moved)} onClick={resetZoom}>
				<RotateCcwIcon size={18} />
			</button>

			<svg ref={svgRef} style={{ display: "block" }} viewBox={"0 0 800 500"}>
				<title>WoldMap</title>
				<g>{landmasses}</g>
			</svg>

			<Tooltip id="map" className={styles.tooltipContainer} classNameArrow={styles.reset} disableStyleInjection>
				{currentLocation && (
					<div className={styles.tooltip} data-theme="dark">
						<h2>{metricNames[metric]}</h2>
						<h3>
							{currentTooltip?.name} <span>{currentTooltip?.value}</span>
						</h3>
					</div>
				)}
			</Tooltip>
		</div>
	);
};

const Landmass = ({
	feature,
	countries,
	biggest,
	onSetLocation,
}: {
	feature: (typeof features)[number];
	countries: Map<string, number>;
	biggest?: DimensionTableRow;
	onSetLocation: (location: Location | null) => void;
}) => {
	const percent = useMemo(
		() => (countries.get(feature.iso) ?? 0) / (biggest?.value ?? 100),
		[countries, feature.iso, biggest],
	);

	return (
		<path
			d={feature.path || ""}
			className={styles.geo}
			style={{ "--percent": percent } as React.CSSProperties}
			onMouseEnter={() => onSetLocation({ name: feature.name, iso: feature.iso })}
			onMouseLeave={() => onSetLocation(null)}
		/>
	);
};
