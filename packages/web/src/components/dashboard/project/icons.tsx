import styles from "./icons.module.css";

import { useState } from "react";
// biome-ignore format: no
import { faAndroid, faApple, faBluesky, faChrome, faDribbble, faEdge, faFacebook, faFirefox, faFlickr, faFoursquare, faGithub, faGoogle, faInstagram, faLastfm, faLinkedin, faLinux, faMastodon, faOdnoklassniki, faOpera, faPinterest, faPixelfed, faQq, faReddit, faRenren, faSafari, faSnapchat, faStackOverflow, faTelegram, faThreads, faTiktok, faTumblr, faTwitch, faTwitter, faViadeo, faVimeo, faVk, faWeibo, faWindows, faXing, faYCombinator, faYoutube } from "@fortawesome/free-brands-svg-icons";
// biome-ignore format: no
import { AppWindowIcon, EarthIcon, MonitorIcon, RectangleHorizontalIcon, RectangleVerticalIcon, SearchIcon, SmartphoneIcon, TabletIcon } from "lucide-react";

import { BrandIcon } from "@/components/ui/brand-icon";
import { appPath, runtimeConfig } from "@/config";

// biome-ignore format: no
const brandIcons = { bluesky: faBluesky, dribbble: faDribbble, facebook: faFacebook, flickr: faFlickr, foursquare: faFoursquare, github: faGithub, google: faGoogle, instagram: faInstagram, lastdotfm: faLastfm, linkedin: faLinkedin, mastodon: faMastodon, odnoklassniki: faOdnoklassniki, pinterest: faPinterest, pixelfed: faPixelfed, reddit: faReddit, renren: faRenren, sinaweibo: faWeibo, snapchat: faSnapchat, stackoverflow: faStackOverflow, telegram: faTelegram, tencentqq: faQq, threads: faThreads, tiktok: faTiktok, tumblr: faTumblr, twitch: faTwitch, viadeo: faViadeo, vimeo: faVimeo, vk: faVk, x: faTwitter, xing: faXing, ycombinator: faYCombinator, youtube: faYoutube };

const genericIcons = {
	search: SearchIcon,
};

const countryFlags = import.meta.glob<string>("../../../../node_modules/country-flag-icons/3x2/[A-Z][A-Z].svg", {
	eager: true,
	import: "default",
	query: "?url&no-inline",
});

export const CountryFlag = ({ code }: { code?: string | null }) => {
	const src = countryFlags[`../../../../node_modules/country-flag-icons/3x2/${code?.toUpperCase()}.svg`];
	return src ? (
		<img className={styles.countryFlag} src={src} width={18} height={12} alt="" loading="lazy" />
	) : (
		<EarthIcon size={18} aria-hidden="true" />
	);
};

const browserIcons = {
	edge: [faEdge, "#0078D7", "#0078D7"],
	chrome: [faChrome, "#ffffff", "#000000"],
	firefox: [faFirefox, "#FF7139", "#FF7139"],
	safari: [faSafari, "#1E90FF", "#1E90FF"],
	opera: [faOpera, "#FF4B2B", "#FF1B2D"],
} as const;
const browsers = Object.keys(browserIcons);

const osIcons = {
	android: [faAndroid, "#3DDC84", "#3DDC84"],
	ios: [faApple, "#FFFFFF", "#000000"],
	macos: [faApple, "#FFFFFF", "#000000"],
	linux: [faLinux, "#FCC624", "#000000"],
	windows: [faWindows, "#0078D7", "#0078D7"],
} as const;
const oses = Object.keys(osIcons);

const deviceIcons = {
	phone: SmartphoneIcon,
	tablet: TabletIcon,
	desktop: MonitorIcon,
};

type IconProps = {
	size?: number;
	color?: string;
};

export const BrowserIcon = ({ browser, ...props }: { browser: string } & IconProps) => {
	for (const b of browsers) {
		if (browser.toLowerCase().replaceAll(" ", "").includes(b)) {
			const [icon, dark, light] = browserIcons[b as keyof typeof browserIcons];
			return (
				<BrandIcon
					{...props}
					icon={icon}
					style={{ "--dark": dark, "--light": light } as React.CSSProperties}
					className={styles.icon}
				/>
			);
		}
	}

	return <EarthIcon {...props} />;
};

export const OSIcon = ({ os, ...props }: { os: string } & IconProps) => {
	const name = os.toLowerCase().replaceAll(" ", "");
	for (const o of oses) {
		if (name.includes(o)) {
			const [icon, dark, light] = osIcons[o as keyof typeof osIcons];
			return (
				<BrandIcon
					{...props}
					icon={icon}
					style={{ "--dark": dark, "--light": light } as React.CSSProperties}
					className={styles.icon}
				/>
			);
		}
	}
	if (name.includes("kindle")) return <TabletIcon {...props} />;
	return <AppWindowIcon {...props} />;
};

export const MobileDeviceIcon = ({ isMobile, ...props }: { isMobile: boolean } & IconProps) => {
	const Icon = isMobile ? deviceIcons.phone : deviceIcons.desktop;
	return <Icon {...props} />;
};

export const OrientationIcon = ({ orientation, ...props }: { orientation: string } & IconProps) => {
	const value = orientation.toLowerCase();
	if (value === "portrait") {
		return <RectangleVerticalIcon {...props} />;
	}

	if (value === "landscape") {
		return <RectangleHorizontalIcon {...props} />;
	}

	return <MonitorIcon {...props} />;
};

export const ReferrerIcon = ({
	referrer,
	icon,
	...props
}: { referrer: string; icon: string | null | undefined } & IconProps) => {
	if (referrer === "liwan.dev" || referrer === "www.liwan.dev") {
		const size = props.size ?? 24;
		return <img src={appPath("/favicon.svg")} alt="" width={size} height={size} />;
	}

	if (icon && Object.hasOwn(brandIcons, icon)) {
		return <BrandIcon {...props} icon={brandIcons[icon as keyof typeof brandIcons]} />;
	}

	if (icon && Object.hasOwn(genericIcons, icon)) {
		const Icon = genericIcons[icon as keyof typeof genericIcons];
		return <Icon {...props} />;
	}

	if (referrer === "Unknown") {
		return <SearchIcon {...props} />;
	}

	return <Favicon {...props} fqdn={referrer} />;
};

export const Favicon = ({
	size,
	fqdn,
}: IconProps & {
	fqdn: string;
}) => {
	const [failedFqdn, setFailedFqdn] = useState<string>();
	const sanitizedFqdn = fqdn.replace(/[^a-zA-Z0-9.-]/g, "");
	if (runtimeConfig?.disableFavicons || !sanitizedFqdn || failedFqdn === sanitizedFqdn) {
		return <EarthIcon size={size} />;
	}

	return (
		<img
			src={`https://icons.duckduckgo.com/ip3/${sanitizedFqdn}.ico`}
			alt=""
			height={size}
			width={size}
			onError={() => setFailedFqdn(sanitizedFqdn)}
			onLoad={(event) => {
				if (event.currentTarget.naturalWidth === 0) setFailedFqdn(sanitizedFqdn);
			}}
		/>
	);
};
