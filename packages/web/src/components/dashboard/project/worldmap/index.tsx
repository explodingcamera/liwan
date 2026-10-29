import tooltipStyles from "../hover-tooltip.module.css";
import styles from "./map.module.css";

import type { CSSProperties, MouseEvent } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { select } from "d3-selection";
import type { ZoomBehavior } from "d3-zoom";
import { zoom as d3Zoom, zoomIdentity } from "d3-zoom";
import { autoUpdate, FloatingPortal, flip, offset, shift, useFloating } from "@floating-ui/react";
import { RotateCcwIcon } from "lucide-react";

import type { DimensionTableRow, Metric } from "@/constants";
import { metricNames } from "@/constants";
import { cls, formatMetricVal } from "@/utils";
import geo from "../../../../../../../data/geo.json";
import { createMapFeatures, type MapTopology } from "./paths";

const features = createMapFeatures(geo as MapTopology);
const initialZoom = zoomIdentity.translate(400, 250).scale(1.25).translate(-400, -250);

type Location = {
	name: string;
	iso: string;
};

const getCountries = (data: DimensionTableRow[]) => {
	const countries = new Map<string, number>();
	for (const row of data) {
		if (row.dimensionValue) countries.set(row.dimensionValue, row.value);
	}
	return countries;
};

export const Worldmap = ({ metric, data }: { metric: Metric; data?: DimensionTableRow[] }) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const containerRef = useRef<HTMLDivElement | null>(null);

	const [moved, setMoved] = useState(false);
	const movedRef = useRef(false);
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
	useEffect(() => {
		const svgElement = svgRef.current;
		if (!svgElement) return;

		const behavior = d3Zoom<SVGSVGElement, unknown>()
			.scaleExtent([0.8, 8])
			.on("zoom", (event) => {
				select(svgElement).select("g").attr("transform", event.transform);
				const hasMoved =
					event.transform.k !== initialZoom.k ||
					event.transform.x !== initialZoom.x ||
					event.transform.y !== initialZoom.y;
				if (movedRef.current !== hasMoved) {
					movedRef.current = hasMoved;
					setMoved(hasMoved);
				}
			});
		zoomBehavior.current = behavior;
		select(svgElement).call(behavior).call(behavior.transform, initialZoom);

		return () => {
			select(svgElement).on(".zoom", null);
			behavior.on("zoom", null);
			zoomBehavior.current = null;
		};
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
			select(svgRef.current).call(zoomBehavior.current.transform, initialZoom);
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
				<title>World map</title>
				<g>{landmasses}</g>
			</svg>

			{currentLocation && (
				<FloatingPortal>
					<div ref={refs.setFloating} className={styles.tooltipContainer} style={floatingStyles}>
						<div className={tooltipStyles.tooltip}>
							<h2>{currentTooltip?.name}</h2>
							<h3>
								<span>{metricNames[metric]}</span> <span>{currentTooltip?.value}</span>
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
