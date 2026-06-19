// Based on https://github.com/d3/d3-axis/blob/main/src/axis.js,
// which is licensed under the ISC License (ISC)
// Modified to allow disabling the domain line and tick lines
// biome-ignore lint: no
type ANY = any;

const identity = (d: ANY) => d;
const top = 1;
const right = 2;
const bottom = 3;
const left = 4;
const epsilon = 1e-6;

const translateX = (x: number) => `translate(${x},0)`;
const translateY = (y: number) => `translate(0,${y})`;
const number = (scale: ANY) => (d: ANY) => +scale(d);

function center(scale: ANY, offset: ANY) {
	let _offset = Math.max(0, scale.bandwidth() - offset * 2) / 2;
	if (scale.round()) _offset = Math.round(_offset);
	return (d: ANY) => +scale(d) + _offset;
}

function entering() {
	// @ts-expect-error
	return !this.__axis;
}

function axis<Domain>(orient: number, scale: ANY) {
	type Axis = typeof axis;

	let tickArguments: unknown[] = [];
	let tickValues: ANY = null;
	let tickFormat: ANY = null;
	let tickSizeInner = 6;
	let tickSizeOuter = 6;
	let tickPadding = 3;
	let offset = typeof window !== "undefined" && window.devicePixelRatio > 1 ? 0 : 0.5;
	const k = orient === top || orient === left ? -1 : 1;
	const x = orient === left || orient === right ? "x" : "y";
	const transform = orient === top || orient === bottom ? translateX : translateY;

	let disableDomain = false;
	let disableTicks = false;

	function axis(context: ANY) {
		const values =
			tickValues == null ? (scale.ticks ? scale.ticks.apply(scale, tickArguments) : scale.domain()) : tickValues;
		const format =
			tickFormat == null ? (scale.tickFormat ? scale.tickFormat.apply(scale, tickArguments) : identity) : tickFormat;
		const spacing = Math.max(tickSizeInner, 0) + tickPadding;
		const range = scale.range();
		const range0 = +range[0] + offset;
		const range1 = +range[range.length - 1] + offset;
		const position = (scale.bandwidth ? center : number)(scale.copy(), offset);
		const selection = context.selection ? context.selection() : context;
		let path = selection.selectAll(".domain").data([null]);
		let tick = selection.selectAll(".tick").data(values, scale).order();
		let tickExit = tick.exit();
		const tickEnter = tick.enter().append("g").attr("class", "tick");
		let line = tick.select("line");
		let text = tick.select("text");

		if (!disableDomain)
			path = path.merge(path.enter().insert("path", ".tick").attr("class", "domain").attr("stroke", "currentColor"));

		tick = tick.merge(tickEnter);

		if (!disableTicks)
			line = line.merge(
				tickEnter
					.append("line")
					.attr("stroke", "currentColor")
					.attr(`${x}2`, k * tickSizeInner),
			);

		text = text.merge(
			tickEnter
				.append("text")
				.attr("fill", "currentColor")
				.attr(x, k * spacing)
				.attr("dy", orient === top ? "0em" : orient === bottom ? "0.71em" : "0.32em"),
		);

		if (context !== selection) {
			path = path.transition(context);
			tick = tick.transition(context);
			line = line.transition(context);
			text = text.transition(context);

			tickExit = tickExit
				.transition(context)
				.attr("opacity", epsilon)
				.attr("transform", function (this: ANY, d: ANY) {
					const _d = position(d);
					return Number.isFinite(_d) ? transform(_d + offset) : <ANY>this.getAttribute("transform");
				});

			tickEnter.attr("opacity", epsilon).attr("transform", function (this: ANY, d: ANY) {
				let p = <ANY>this.parentNode.__axis;
				let t = 0;

				if (p) {
					p = p(d);
					t = Number.isFinite(p) ? p : position(d);
				}

				return transform(t + offset);
			});
		}

		tickExit.remove();

		path.attr(
			"d",
			orient === left || orient === right
				? tickSizeOuter
					? `M${k * tickSizeOuter},${range0}H${offset}V${range1}H${k * tickSizeOuter}`
					: `M${offset},${range0}V${range1}`
				: tickSizeOuter
					? `M${range0},${k * tickSizeOuter}V${offset}H${range1}V${k * tickSizeOuter}`
					: `M${range0},${offset}H${range1}`,
		);

		tick.attr("opacity", 1).attr("transform", (d: ANY) => transform(position(d) + offset));

		line.attr(`${x}2`, k * tickSizeInner);

		text.attr(x, k * spacing).text(format);

		selection
			.filter(entering)
			.attr("fill", "none")
			.attr("font-size", 10)
			.attr("font-family", "sans-serif")
			.attr("text-anchor", orient === right ? "start" : orient === left ? "end" : "middle");

		selection.each(function (this: ANY) {
			this.__axis = position;
		});
	}

	axis.disableDomain = () => {
		disableDomain = true;
		return axis as Axis;
	};

	axis.disableTicks = () => {
		disableTicks = true;
		return axis as Axis;
	};

	axis.ticks = (...args: ANY[]) => {
		tickArguments = args;
		return axis as Axis;
	};

	axis.tickArguments = (...args: ANY[]) => {
		if (args.length) {
			tickArguments = args[0] == null ? [] : Array.from(args[0]);
			return axis as Axis;
		}
		return tickArguments.slice();
	};

	axis.tickValues = (...args: ANY[]) => {
		if (args.length) {
			tickValues = args[0] == null ? null : Array.from(args[0]);
			return axis as Axis;
		}
		return tickValues?.slice();
	};

	axis.tickFormat = (format: (domainValue: Domain, index: number) => string) => {
		tickFormat = format;
		return axis as Axis;
	};

	axis.tickSize = (...args: ANY[]) => {
		if (args.length) {
			tickSizeInner = tickSizeOuter = +args[0];
			return axis as Axis;
		}

		return tickSizeInner;
	};

	axis.tickSizeInner = (...args: ANY[]) => {
		if (args.length) {
			tickSizeInner = +args[0];
			return axis as Axis;
		}
		return tickSizeInner;
	};

	axis.tickSizeOuter = (...args: ANY[]) => {
		if (args.length) {
			tickSizeOuter = +args[0];
			return axis as Axis;
		}
		return tickSizeOuter;
	};

	axis.tickPadding = (...args: ANY[]) => {
		if (args.length) {
			tickPadding = +args[0];
			return axis as Axis;
		}
		return tickPadding;
	};

	axis.offset = (...args: ANY[]) => {
		if (args.length) {
			offset = +args[0];
			return axis as Axis;
		}
		return offset;
	};

	return axis as Axis;
}

export type AxisDomain = number | string | Date | { valueOf(): number };
export interface AxisScale<Domain> {
	(x: Domain): number | undefined;
	domain(): Domain[];
	range(): number[];
	copy(): this;
	bandwidth?(): number;
}

export function axisTop<Domain extends AxisDomain>(scale: AxisScale<Domain>) {
	return axis<Domain>(top, scale);
}

export function axisRight<Domain extends AxisDomain>(scale: AxisScale<Domain>) {
	return axis<Domain>(right, scale);
}

export function axisBottom<Domain extends AxisDomain>(scale: AxisScale<Domain>) {
	return axis<Domain>(bottom, scale);
}

export function axisLeft<Domain extends AxisDomain>(scale: AxisScale<Domain>) {
	return axis<Domain>(left, scale);
}
