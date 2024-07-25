import {
	SiTencentqq,
	SiFoursquare,
	SiVk,
	SiSinaweibo,
	SiTelegram,
	SiPixelfed,
	SiWorkplace,
	SiX,
	SiThreads,
	SiMaildotru,
	SiYcombinator,
	SiTiktok,
	SiFacebook,
	SiLastdotfm,
	SiLinkedin,
	SiDribbble,
	SiReddit,
	SiFlickr,
	SiGithub,
	SiPinterest,
	SiSkyrock,
	SiStackoverflow,
	SiBluesky,
	SiLivejournal,
	SiV2ex,
	SiDouban,
	SiRenren,
	SiTumblr,
	SiSnapchat,
	SiBadoo,
	SiYoutube,
	SiInstagram,
	SiViadeo,
	SiOdnoklassniki,
	SiVimeo,
	SiMastodon,
	SiSourceforge,
	SiTwitch,
	SiXing,
	SiGoogle,
	SiDuckduckgo,
	SiGooglechrome,
	SiFirefox,
	SiSafari,
	SiOpera,
	SiAndroid,
	SiIos,
	SiMacos,
	SiLinux,
} from "@icons-pack/react-simple-icons";
import {
	AppWindowIcon,
	EarthIcon,
	LayoutGridIcon,
	MonitorIcon,
	SearchIcon,
	SmartphoneIcon,
	TabletIcon,
} from "lucide-react";

const brandIcons = {
	tencentqq: SiTencentqq,
	foursquare: SiFoursquare,
	vk: SiVk,
	sinaweibo: SiSinaweibo,
	telegram: SiTelegram,
	pixelfed: SiPixelfed,
	workplace: SiWorkplace,
	x: SiX,
	threads: SiThreads,
	Ru: SiMaildotru,
	News: SiYcombinator,
	tiktok: SiTiktok,
	facebook: SiFacebook,
	lastdotfm: SiLastdotfm,
	linkedin: SiLinkedin,
	dribbble: SiDribbble,
	reddit: SiReddit,
	flickr: SiFlickr,
	github: SiGithub,
	pinterest: SiPinterest,
	skyrock: SiSkyrock,
	stackoverflow: SiStackoverflow,
	bluesky: SiBluesky,
	livejournal: SiLivejournal,
	v2ex: SiV2ex,
	douban: SiDouban,
	renren: SiRenren,
	tumblr: SiTumblr,
	snapchat: SiSnapchat,
	badoo: SiBadoo,
	youtube: SiYoutube,
	instagram: SiInstagram,
	viadeo: SiViadeo,
	odnoklassniki: SiOdnoklassniki,
	vimeo: SiVimeo,
	mastodon: SiMastodon,
	sourceforge: SiSourceforge,
	twitch: SiTwitch,
	xing: SiXing,
	google: SiGoogle,
	duckduckgo: SiDuckduckgo,
};

const genericIcons = {
	search: SearchIcon,
};

// microsoft icons are not included in simple-icons
const browserIcons = {
	chrome: SiGooglechrome,
	firefox: SiFirefox,
	safari: SiSafari,
	opera: SiOpera,
};

const osIcons = {
	android: SiAndroid,
	ios: SiIos,
	macos: SiMacos,
	linux: SiLinux,
};

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
	if (Object.hasOwnProperty.call(browserIcons, browser)) {
		const Icon = browserIcons[browser as keyof typeof browserIcons];
		return <Icon {...props} />;
	}
	return <EarthIcon {...props} />;
};

export const OSIcon = ({ os, ...props }: { os: string } & IconProps) => {
	if (Object.hasOwnProperty.call(osIcons, os)) {
		const Icon = osIcons[os as keyof typeof osIcons];
		return <Icon {...props} />;
	}
	if (os === "windows") {
		return <LayoutGridIcon {...props} />;
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
