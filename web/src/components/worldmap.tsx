import styles from "./worldmap.module.css";
import geo from "../../../data/geo.json";
import { ComposableMap, Geographies, Geography } from "react-simple-maps";
import { Tooltip } from "react-tooltip";
import { useState } from "react";

type Geo = {
	name: string;
	iso: string;
};

export const WorldMap = () => {
	const [currentGeo, setCurrentGeo] = useState<Geo | null>(null);

	return (
		<div className={styles.worldmap} data-tooltip-float={true} data-tooltip-id="map">
			<ComposableMap projection="geoMercator">
				<Geographies geography={geo}>
					{({ geographies }) =>
						geographies.map((geo) => {
							console.log(geo);

							return (
								<Geography
									className={styles.geo}
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
			</ComposableMap>
			<Tooltip id="map" className={styles.reset} classNameArrow={styles.reset} disableStyleInjection>
				{currentGeo && (
					<div className={styles.tooltip} data-theme="dark">
						<h2>{currentGeo.name}</h2>
						<h3>
							{currentGeo.iso} <span>asdf</span>
						</h3>
					</div>
				)}
			</Tooltip>
		</div>
	);
};
