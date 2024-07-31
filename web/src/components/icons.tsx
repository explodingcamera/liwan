import styles from "./icons.module.css";

// biome-ignore format:
import { SiTencentqq, SiFoursquare, SiVk, SiSinaweibo, SiTelegram, SiPixelfed, SiWorkplace, SiX, SiThreads, SiMaildotru, SiYcombinator, SiTiktok, SiFacebook, SiLastdotfm, SiLinkedin, SiDribbble, SiReddit, SiFlickr, SiGithub, SiPinterest, SiSkyrock, SiStackoverflow, SiBluesky, SiLivejournal, SiV2ex, SiDouban, SiRenren, SiTumblr, SiSnapchat, SiBadoo, SiYoutube, SiInstagram, SiViadeo, SiOdnoklassniki, SiVimeo, SiMastodon, SiSourceforge, SiTwitch, SiXing, SiGoogle, SiDuckduckgo, SiGooglechrome, SiFirefox, SiSafari, SiOpera, SiAndroid, SiIos, SiMacos, SiLinux } from "@icons-pack/react-simple-icons";
// biome-ignore format:
import { AppWindowIcon, EarthIcon, LayoutGridIcon, MonitorIcon, SearchIcon, SmartphoneIcon, TabletIcon } from "lucide-react";
// biome-ignore format:
const brandIcons = { tencentqq: SiTencentqq, foursquare: SiFoursquare, vk: SiVk, sinaweibo: SiSinaweibo, telegram: SiTelegram, pixelfed: SiPixelfed, workplace: SiWorkplace, x: SiX, threads: SiThreads, Ru: SiMaildotru, News: SiYcombinator, tiktok: SiTiktok, facebook: SiFacebook, lastdotfm: SiLastdotfm, linkedin: SiLinkedin, dribbble: SiDribbble, reddit: SiReddit, flickr: SiFlickr, github: SiGithub, pinterest: SiPinterest, skyrock: SiSkyrock, stackoverflow: SiStackoverflow, bluesky: SiBluesky, livejournal: SiLivejournal, v2ex: SiV2ex, douban: SiDouban, renren: SiRenren, tumblr: SiTumblr, snapchat: SiSnapchat, badoo: SiBadoo, youtube: SiYoutube, instagram: SiInstagram, viadeo: SiViadeo, odnoklassniki: SiOdnoklassniki, vimeo: SiVimeo, mastodon: SiMastodon, sourceforge: SiSourceforge, twitch: SiTwitch, xing: SiXing, google: SiGoogle, duckduckgo: SiDuckduckgo,};

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

export const ReferrerIcon = ({ referrer, icon, ...props }: { referrer: string; icon: string } & IconProps) => {
	if (Object.hasOwnProperty.call(brandIcons, icon)) {
		const Icon = brandIcons[icon as keyof typeof brandIcons];
		return <Icon {...props} />;
	}

	if (Object.hasOwnProperty.call(genericIcons, icon)) {
		const Icon = genericIcons[icon as keyof typeof genericIcons];
		return <Icon {...props} />;
	}

	<Favicon {...props} />;
};

export const Favicon = ({ size }: IconProps) => {
	return <img src={`https://icons.duckduckgo.com/ip3/${size}.ico`} alt="favicon" height={size} width={size} />;
};
