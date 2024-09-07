import styles from "./worldmap.module.css";

import { useMemo, useState } from "react";
import { Tooltip } from "react-tooltip";
import { ComposableMap, Geographies, Geography, ZoomableGroup } from "react-simple-maps";

import { type DimensionTableRow, type Metric, metricNames } from "../api";
import { formatMetricVal } from "../utils";
import geo from "../../../data/geo.json";

type Geo = {
	name: string;
	iso: string;
};

export const WorldMap = ({
	metric,
	data,
}: {
	metric: Metric;
	data?: DimensionTableRow[];
}) => {
	const [currentGeo, setCurrentGeo] = useState<Geo | null>(null);

	const countries = useMemo(() => {
		const countries = new Map<string, number>();
		for (const row of data ?? []) {
			countries.set(row.dimensionValue, row.value);
		}
		return countries;
	}, [data]);

	const biggest = useMemo(() => data?.reduce((a, b) => (a.value > b.value ? a : b), data[0]), [data]);

	return (
		<div className={styles.worldmap} data-tooltip-float={true} data-tooltip-id="map">
			<ComposableMap
				projection="geoMercator"
				projectionConfig={{
					rotate: [0, 0, 0],
					center: [0, 50],
					scale: 120,
				}}
				height={500}
			>
				<ZoomableGroup>
					<Geographies geography={geo}>
						{({ geographies }) =>
							geographies.map((geo) => {
								return (
									<Geography
										className={styles.geo}
										style={{
											default: {
												"--percent": (countries.get(geo.properties.iso) ?? 0) / (biggest?.value ?? 100),
											} as React.CSSProperties,
											hover: {
												"--percent": (countries.get(geo.properties.iso) ?? 0) / (biggest?.value ?? 100),
											} as React.CSSProperties,
											pressed: {
												"--percent": (countries.get(geo.properties.iso) ?? 0) / (biggest?.value ?? 100),
											} as React.CSSProperties,
										}}
										onMouseEnter={() => {
											setCurrentGeo({
												name: geo.properties.name,
												iso: geo.properties.iso,
											});
										}}
										onMouseLeave={() => {
											setCurrentGeo(null);
										}}
										key={geo.rsmKey}
										geography={geo}
									/>
								);
							})
						}
					</Geographies>
				</ZoomableGroup>
			</ComposableMap>
			<Tooltip id="map" className={styles.tooltipContainer} classNameArrow={styles.reset} disableStyleInjection>
				{currentGeo && (
					<div className={styles.tooltip} data-theme="dark">
						<h2>{metricNames[metric]}</h2>
						<h3>
							{currentGeo.name} <span>{formatMetricVal(metric, countries.get(currentGeo.iso) ?? 0)}</span>
						</h3>
					</div>
				)}
			</Tooltip>
		</div>
	);
};
