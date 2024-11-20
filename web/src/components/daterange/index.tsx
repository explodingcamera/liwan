import styles from "./daterange.module.css";

export const DatePickerRange = () => {
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
		</div>
	);
};
