import tooltipStyles from "../hover-tooltip.module.css";
import styles from "./map.module.css";

import type { CSSProperties, MouseEvent } from "react";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { select } from "d3-selection";
import type { D3ZoomEvent, ZoomBehavior } from "d3-zoom";
import { zoom as d3Zoom, zoomIdentity } from "d3-zoom";
import { autoUpdate, FloatingPortal, flip, offset, shift, useFloating } from "@floating-ui/react";
import { RotateCcwIcon } from "lucide-react";

import type { DimensionTableRow, Metric } from "@/constants";
import { metricNames } from "@/constants";
import { cls } from "@/utils";
import geo from "../../../../../../../data/geo.json";
import { formatMetricVal } from "../metric-format";
import { createMapFeatures, lensOffset, lensScale, type MapTopology, mapSize } from "./paths";

const features = createMapFeatures(geo as MapTopology);
const initialZoom = zoomIdentity
	.translate(mapSize.width / 2, mapSize.height / 2)
	.scale(1.25)
	.translate(-mapSize.width / 2, -mapSize.height / 2);

type Location = {
	name: string;
	iso: string;
};

const getCountries = (data: DimensionTableRow[]) => {
	const countries = new Map<string, number | null>();
	for (const row of data) {
		if (row.dimensionValue) countries.set(row.dimensionValue, row.value ?? null);
	}
	return countries;
};

export const Worldmap = ({ metric, data }: { metric: Metric; data?: DimensionTableRow[] }) => {
	const svgRef = useRef<SVGSVGElement | null>(null);
	const countriesRef = useRef<SVGGElement | null>(null);
	const displacementRef = useRef<SVGFEDisplacementMapElement | null>(null);
	const displacementScale = useRef(lensScale);
	const panning = useRef(false);
	const lensId = useId();
	const [lensTexture, setLensTexture] = useState<string>();

	useEffect(() => {
		const canvas = document.createElement("canvas");
		canvas.width = 64;
		canvas.height = 40;
		const context = canvas.getContext("2d");
		if (!context) return;

		const image = context.createImageData(canvas.width, canvas.height);
		for (let y = 0; y < canvas.height; y++) {
			for (let x = 0; x < canvas.width; x++) {
				const offset = lensOffset(
					((x + 0.5) * mapSize.width) / canvas.width,
					((y + 0.5) * mapSize.height) / canvas.height,
				);
				const index = (y * canvas.width + x) * 4;
				image.data[index] = (0.5 + offset.x) * 255;
				image.data[index + 1] = (0.5 + offset.y) * 255;
				image.data[index + 3] = 255;
			}
		}
		context.putImageData(image, 0, 0);
		setLensTexture(canvas.toDataURL());
	}, []);

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

	const biggest = useMemo(() => data?.reduce((max, row) => Math.max(max, row.value ?? 0), 0) ?? 0, [data]);
	const countries = useMemo(() => getCountries(data ?? []), [data]);

	const zoomBehavior = useRef<ZoomBehavior<SVGSVGElement, unknown>>(null);
	useEffect(() => {
		const svgElement = svgRef.current;
		const countriesLayer = countriesRef.current;
		if (!svgElement || !countriesLayer) return;

		let frame = 0;
		const behavior = d3Zoom<SVGSVGElement, unknown>()
			.scaleExtent([0.8, 8])
			.on("start", ({ sourceEvent }: D3ZoomEvent<SVGSVGElement, unknown>) => {
				if (sourceEvent?.type !== "mousedown" && sourceEvent?.type !== "touchstart") return;
				panning.current = true;
				setCurrentLocation(null);
			})
			.on("zoom", ({ transform }: D3ZoomEvent<SVGSVGElement, unknown>) => {
				cancelAnimationFrame(frame);
				frame = requestAnimationFrame(() => {
					countriesLayer.setAttribute("transform", transform.toString());
					const scale = lensScale * Math.max(1, initialZoom.k / transform.k);
					if (scale !== displacementScale.current) {
						displacementScale.current = scale;
						displacementRef.current?.setAttribute("scale", String(scale));
					}
				});
				const hasMoved =
					transform.k !== initialZoom.k || transform.x !== initialZoom.x || transform.y !== initialZoom.y;
				if (movedRef.current !== hasMoved) {
					movedRef.current = hasMoved;
					setMoved(hasMoved);
				}
			})
			.on("end", () => {
				panning.current = false;
			});
		zoomBehavior.current = behavior;
		select(svgElement).call(behavior).call(behavior.transform, initialZoom);

		return () => {
			cancelAnimationFrame(frame);
			select(svgElement).on(".zoom", null);
			behavior.on("start zoom end", null);
			panning.current = false;
			zoomBehavior.current = null;
		};
	}, []);

	const moveTooltip = (event: MouseEvent<SVGSVGElement>) => {
		if (panning.current) return;
		const { clientX, clientY, currentTarget: svg } = event;
		let point = new DOMPoint(clientX, clientY);
		const matrix = lensTexture && svg.getScreenCTM();
		if (matrix) {
			// SVG filters leave hit targets unchanged. Sample the same source position as the lens.
			const local = point.matrixTransform(matrix.inverse());
			if (local.x < 0 || local.x > mapSize.width || local.y < 0 || local.y > mapSize.height) {
				setCurrentLocation(null);
				return;
			}
			const offset = lensOffset(local.x, local.y);
			point = new DOMPoint(
				local.x + displacementScale.current * offset.x,
				local.y + displacementScale.current * offset.y,
			).matrixTransform(matrix);
		}
		const path = document.elementFromPoint(point.x, point.y)?.closest<SVGPathElement>("[data-map-country]");
		const location = path && svg.contains(path) ? features[Number(path.dataset.mapCountry)] : null;
		setCurrentLocation(location);
		if (!location) return;
		refs.setPositionReference({ getBoundingClientRect: () => new DOMRect(clientX, clientY, 0, 0) });
		update();
	};

	const landmasses = useMemo(
		() =>
			features.map((feature, index) => {
				const value = countries.get(feature.iso);
				return (
					<path
						key={index}
						d={feature.path}
						className={styles.geo}
						data-map-country={index}
						data-inverted={metric === "bounce_rate"}
						data-ignored={value == null}
						style={{ "--percent": biggest > 0 ? (value ?? 0) / biggest : 0 } as CSSProperties}
					/>
				);
			}),
		[countries, biggest, metric],
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
				value: formatMetricVal(countries.get(currentLocation.iso), metric),
			},
		[currentLocation, countries, metric],
	);

	return (
		<div className={styles.worldmap}>
			<button type="button" className={cls(styles.reset, moved && styles.moved)} onClick={resetZoom}>
				<RotateCcwIcon size={18} />
			</button>

			<svg
				ref={svgRef}
				viewBox={`0 0 ${mapSize.width} ${mapSize.height}`}
				preserveAspectRatio="xMidYMid slice"
				onMouseMove={moveTooltip}
				onMouseLeave={() => setCurrentLocation(null)}
			>
				<title>World map</title>
				<defs>
					<filter id={lensId} x={0} y={0} {...mapSize} filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
						<feImage href={lensTexture} {...mapSize} preserveAspectRatio="none" result="lens" />
						<feDisplacementMap
							ref={displacementRef}
							in="SourceGraphic"
							in2="lens"
							scale={lensScale}
							xChannelSelector="R"
							yChannelSelector="G"
						/>
					</filter>
				</defs>
				<g filter={lensTexture ? `url(#${lensId})` : undefined}>
					<g ref={countriesRef} transform={initialZoom.toString()}>
						{landmasses}
					</g>
				</g>
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
