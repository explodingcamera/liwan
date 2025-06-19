import { useConfig } from "../api";
import styles from "./icons.module.css";

// biome-ignore format: no
import { SiAndroid, SiBadoo, SiBluesky, SiDouban, SiDribbble, SiDuckduckgo, SiFacebook, SiFirefox, SiFlickr, SiFoursquare, SiGithub, SiGoogle, SiGooglechrome, SiInstagram, SiIos, SiLastdotfm, SiLinux, SiLivejournal, SiMacos, SiMaildotru, SiMastodon, SiOdnoklassniki, SiOpera, SiPinterest, SiPixelfed, SiReddit, SiRenren, SiSafari, SiSinaweibo, SiSnapchat, SiSourceforge, SiStackoverflow, SiTelegram, SiThreads, SiTiktok, SiTumblr, SiTwitch, SiV2ex, SiViadeo, SiVimeo, SiVk, SiWorkplace, SiX, SiXing, SiYcombinator, SiYoutube } from "@icons-pack/react-simple-icons";
// biome-ignore format: no
import { AppWindowIcon, EarthIcon, LayoutGridIcon, MonitorIcon, SearchIcon, SmartphoneIcon, TabletIcon } from "lucide-react";
// biome-ignore format: no
const brandIcons = { foursquare: SiFoursquare, vk: SiVk, sinaweibo: SiSinaweibo, telegram: SiTelegram, pixelfed: SiPixelfed, workplace: SiWorkplace, x: SiX, threads: SiThreads, Ru: SiMaildotru, News: SiYcombinator, tiktok: SiTiktok, facebook: SiFacebook, lastdotfm: SiLastdotfm, dribbble: SiDribbble, reddit: SiReddit, flickr: SiFlickr, github: SiGithub, pinterest: SiPinterest, stackoverflow: SiStackoverflow, bluesky: SiBluesky, livejournal: SiLivejournal, v2ex: SiV2ex, douban: SiDouban, renren: SiRenren, tumblr: SiTumblr, snapchat: SiSnapchat, badoo: SiBadoo, youtube: SiYoutube, instagram: SiInstagram, viadeo: SiViadeo, odnoklassniki: SiOdnoklassniki, vimeo: SiVimeo, mastodon: SiMastodon, sourceforge: SiSourceforge, twitch: SiTwitch, xing: SiXing, google: SiGoogle, duckduckgo: SiDuckduckgo,};

const genericIcons = {
	search: SearchIcon,
};

// microsoft icons are not included in simple-icons
const browserIcons = {
	chrome: [SiGooglechrome, "#ffffff", "#000000"],
	firefox: [SiFirefox, "#FF7139", "#FF7139"],
	safari: [SiSafari, "#1E90FF", "#1E90FF"],
	opera: [SiOpera, "#FF4B2B", "#FF1B2D"],
	edge: [EarthIcon, "#0078D7", "#0078D7"],
};
const browsers = Object.keys(browserIcons);

const osIcons = {
	android: [SiAndroid, "#3DDC84", "#3DDC84"],
	ios: [SiIos, "#FFFFFF", "#000000"],
	macos: [SiMacos, "#FFFFFF", "#000000"],
	linux: [SiLinux, "#FCC624", "#000000"],
	kindle: [TabletIcon, "#FFFFFF", "#000000"],
};
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
			const [Icon, dark, light] = browserIcons[b as keyof typeof browserIcons];
			return (
				<Icon {...props} style={{ "--dark": dark, "--light": light } as React.CSSProperties} className={styles.icon} />
			);
		}
	}

	return <EarthIcon {...props} />;
};

export const OSIcon = ({ os, ...props }: { os: string } & IconProps) => {
	if (os.toLowerCase() === "windows") {
		return <LayoutGridIcon {...props} />;
	}
	for (const o of oses) {
		if (os.toLowerCase().replaceAll(" ", "").includes(o)) {
			const [Icon, dark, light] = osIcons[o as keyof typeof osIcons];
			return (
				<Icon {...props} style={{ "--dark": dark, "--light": light } as React.CSSProperties} className={styles.icon} />
			);
		}
	}
	return <AppWindowIcon {...props} />;
};

export const MobileDeviceIcon = ({ isMobile, ...props }: { isMobile: boolean } & IconProps) => {
	const Icon = isMobile ? deviceIcons.phone : deviceIcons.desktop;
	return <Icon {...props} />;
};

export const ReferrerIcon = ({ referrer, icon, ...props }: { referrer: string; icon?: string } & IconProps) => {
	if (icon && Object.hasOwn(brandIcons, icon)) {
		const Icon = brandIcons[icon as keyof typeof brandIcons];
		return <Icon {...props} />;
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
	const config = useConfig();
	if (config.isLoading || config.config?.disableFavicons) return <SearchIcon size={size} />;

	fqdn = fqdn.replace(/[^a-zA-Z0-9.-]/g, "");
	return <img src={`https://icons.duckduckgo.com/ip3/${fqdn}.ico`} alt="favicon" height={size} width={size} />;
};
