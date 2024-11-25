import styles from "./map.module.css";

import { RotateCcwIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Tooltip } from "react-tooltip";

import { type GeoProjection, geoMercator, geoPath } from "d3-geo";
import { select } from "d3-selection";
import { type ZoomBehavior, zoom as d3Zoom, zoomIdentity } from "d3-zoom";

import type { Feature, Geometry } from "geojson";
import * as topo from "topojson-client";
import type { GeometryCollection, Topology } from "topojson-specification";

import geo from "../../../../data/geo.json";
import { type DimensionTableRow, type Metric, metricNames } from "../../api";
import { cls, formatMetricVal } from "../../utils";

const features = topo.feature(geo as unknown as Topology, geo.objects.geo as GeometryCollection).features as Feature<
	Geometry,
	{
		name: string;
		iso: string;
	}
>[];

type Location = {
	name: string;
	iso: string;
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
	const [dimensions, setDimensions] = useState({ width: 800, height: 400 });
	const [moved, setMoved] = useState(false);

	const [currentLocation, setCurrentLocation] = useState<Location | null>(null);
	const biggest = useMemo(() => data?.reduce((a, b) => (a.value > b.value ? a : b), data[0]), [data]);
	const countries = useMemo(() => {
		const countries = new Map<string, number>();
		for (const row of data ?? []) {
			countries.set(row.dimensionValue, row.value);
		}
		return countries;
	}, [data]);

	const projection = useRef<GeoProjection>();
	if (!projection.current) {
		projection.current = geoMercator()
			.scale((dimensions.width / dimensions.width) * 125)
			// .scale((dimensions.width / 800) * 170)
			// .translate([dimensions.width / 2, dimensions.height / 2])
			.center([45, 45]);
	}
	const pathGenerator = geoPath(projection.current);

	const zoomBehavior = useRef<ZoomBehavior<SVGSVGElement, unknown>>();
	if (!zoomBehavior.current) {
		zoomBehavior.current = d3Zoom<SVGSVGElement, unknown>()
			.scaleExtent([1, 8]) // Min and max zoom levels
			.on("zoom", (event) => {
				select(svgRef.current).select("g").attr("transform", event.transform);
				setMoved(true);
			});
	}

	useEffect(() => {
		if (!svgRef.current || !zoomBehavior.current) return;

		// Setup zoom behavior
		select(svgRef.current).call(zoomBehavior.current);

		const resizeObserver = new ResizeObserver((entries) => {
			if (entries[0].contentRect) {
				const { width, height } = entries[0].contentRect;
				setDimensions({ width, height });
			}
		});

		if (containerRef.current) {
			resizeObserver.observe(containerRef.current);
		}

		return () => {
			if (containerRef.current) {
				resizeObserver.unobserve(containerRef.current);
			}
		};
	}, []);

	return (
		<div ref={containerRef} className={styles.worldmap} data-tooltip-float={true} data-tooltip-id="map">
			<button
				type="button"
				className={cls(styles.reset, moved && styles.moved)}
				onClick={() => {
					setCurrentLocation(null);

					if (zoomBehavior.current && svgRef.current) {
						select(svgRef.current).call(zoomBehavior.current.transform, zoomIdentity);
						setMoved(false);
					}
				}}
			>
				<RotateCcwIcon size={18} />
			</button>

			<svg ref={svgRef} style={{ display: "block" }} viewBox={"0 0 800 500"}>
				<title>WoldMap</title>
				<g>
					{features.map((feature, index) => (
						<Landmass
							key={index}
							feature={feature}
							pathGenerator={pathGenerator}
							countries={countries}
							biggest={biggest}
							onSetLocation={setCurrentLocation}
						/>
					))}
				</g>
			</svg>
			<Tooltip id="map" className={styles.tooltipContainer} classNameArrow={styles.reset} disableStyleInjection>
				{currentLocation && (
					<div className={styles.tooltip} data-theme="dark">
						<h2>{metricNames[metric]}</h2>
						<h3>
							{currentLocation.name} <span>{formatMetricVal(countries.get(currentLocation.iso) ?? 0, metric)}</span>
						</h3>
					</div>
				)}
			</Tooltip>
		</div>
	);
};

const Landmass = ({
	feature,
	pathGenerator,
	countries,
	biggest,
	onSetLocation,
}: {
	feature: Feature<Geometry, { name: string; iso: string }>;
	pathGenerator: ReturnType<typeof geoPath>;
	countries: Map<string, number>;
	biggest?: DimensionTableRow;
	onSetLocation: (location: Location | null) => void;
}) => {
	const path = useMemo(() => pathGenerator(feature), [pathGenerator, feature]);
	const percent = (countries.get(feature.properties.iso) ?? 0) / (biggest?.value ?? 100);

	return (
		<path
			d={path || ""}
			className={styles.geo}
			style={{ "--percent": percent } as React.CSSProperties}
			onMouseEnter={() => onSetLocation({ name: feature.properties.name, iso: feature.properties.iso })}
			onMouseLeave={() => onSetLocation(null)}
		/>
	);
};
