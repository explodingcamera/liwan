import styles from "./daterange.module.css";
import { useId } from "react";

const resolution = {
	hours: "hours",
	days: "days",
	weeks: "weeks",
	months: "months",
};

export const DateRange = () => {
	const start = useId();
	const end = useId();

	return (
		<div className={styles.container}>
			<label>
				Start date:
				<input type="date" id="start" name="trip-start" value="2018-07-22" min="2018-01-01" max="2018-12-31" />
			</label>
			<label>
				End date:
				<input type="date" id="start" name="trip-start" value="2018-07-22" min="2018-01-01" max="2018-12-31" />
			</label>

			<label>
				Resolution:
				<select>
					{Object.entries(resolution).map(([key, value]) => (
						<option key={key} value={key}>
							{value}
						</option>
					))}
				</select>
			</label>
		</div>
	);
};
