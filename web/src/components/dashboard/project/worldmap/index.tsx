import styles from "./map.module.css";

import type { CSSProperties, MouseEvent } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { geoMercator, geoPath } from "d3-geo";
import { select } from "d3-selection";
import type { ZoomBehavior } from "d3-zoom";
import { zoom as d3Zoom, zoomIdentity } from "d3-zoom";
import { autoUpdate, FloatingPortal, flip, offset, shift, useFloating } from "@floating-ui/react";
import { RotateCcwIcon } from "lucide-react";
import * as topo from "topojson-client";
import type { GeometryCollection, Topology } from "topojson-specification";

import type { DimensionTableRow, Metric } from "@/constants";
import { metricNames } from "@/constants";
import { cls, formatMetricVal } from "@/utils";
import geo from "../../../../../../data/geo.json";

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

export const Worldmap = ({ metric, data }: { metric: Metric; data?: DimensionTableRow[] }) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const containerRef = useRef<HTMLDivElement | null>(null);

	const [moved, setMoved] = useState(false);
	const [currentLocation, setCurrentLocation] = useState<Location | null>(null);
	const { refs, floatingStyles, update } = useFloating({
		open: currentLocation !== null,
		placement: "right",
		strategy: "fixed",
		middleware: [offset({ mainAxis: 12, crossAxis: 6 }), flip(), shift({ padding: 8 })],
		whileElementsMounted: autoUpdate,
	});

	const biggest = useMemo(() => data?.reduce((a, b) => (a.value > b.value ? a : b), data[0]), [data]);
	const countries = useMemo(() => getCountries(data ?? []), [data]);

	const zoomBehavior = useRef<ZoomBehavior<SVGSVGElement, unknown>>(null);
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

	const updateTooltipPosition = useCallback(
		(event: MouseEvent<SVGPathElement>) => {
			refs.setPositionReference({
				getBoundingClientRect: () => new DOMRect(event.clientX, event.clientY, 0, 0),
			});
			update();
		},
		[refs, update],
	);

	const landmasses = useMemo(
		() =>
			features.map((feature, index) => (
				<Landmass
					key={index}
					feature={feature}
					countries={countries}
					biggest={biggest}
					onSetLocation={setCurrentLocation}
					onTooltipMove={updateTooltipPosition}
					metric={metric}
				/>
			)),
		[countries, biggest, metric, updateTooltipPosition],
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
		<div ref={containerRef} className={styles.worldmap}>
			<button type="button" className={cls(styles.reset, moved && styles.moved)} onClick={resetZoom}>
				<RotateCcwIcon size={18} />
			</button>

			<svg ref={svgRef} style={{ display: "block" }} viewBox={"0 0 800 500"}>
				<title>WoldMap</title>
				<g>{landmasses}</g>
			</svg>

			{currentLocation && (
				<FloatingPortal>
					<div ref={refs.setFloating} className={styles.tooltipContainer} style={floatingStyles}>
						<div className={styles.tooltip} data-theme="dark">
							<h2>{metricNames[metric]}</h2>
							<h3>
								{currentTooltip?.name} <span>{currentTooltip?.value}</span>
							</h3>
						</div>
					</div>
				</FloatingPortal>
			)}
		</div>
	);
};

const Landmass = ({
	feature,
	countries,
	biggest,
	onSetLocation,
	onTooltipMove,
	metric,
}: {
	feature: (typeof features)[number];
	countries: Map<string, number>;
	biggest?: DimensionTableRow;
	onSetLocation: (location: Location | null) => void;
	onTooltipMove: (event: MouseEvent<SVGPathElement>) => void;
	metric: Metric;
}) => {
	const percent = useMemo(
		() => (countries.get(feature.iso) ?? 0) / (biggest?.value ?? 100),
		[countries, feature.iso, biggest],
	);

	return (
		<path
			d={feature.path || ""}
			className={styles.geo}
			data-inverted={metric === "bounce_rate"}
			data-ignored={metric === "bounce_rate" && (percent === 0 || percent === 100)}
			style={{ "--percent": percent } as CSSProperties}
			onMouseEnter={(event) => {
				onTooltipMove(event);
				onSetLocation({ name: feature.name, iso: feature.iso });
			}}
			onMouseMove={onTooltipMove}
			onMouseLeave={() => onSetLocation(null)}
		/>
	);
};
