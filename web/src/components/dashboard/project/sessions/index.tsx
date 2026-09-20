import styles from "./sessions.module.css";

import { useState } from "react";
import { ChevronDownIcon, ChevronUpIcon, GlobeIcon, RotateCwIcon, UserIcon } from "lucide-react";

import type { SessionEvent, SessionRow } from "@/constants";
import { useProjectSessions, useSessionTimeline } from "@/hooks/api";
import { formatEventTime, type TimeFormat, useTimeFormat } from "@/hooks/persist";
import { cls, countryCodeToFlag, formatPath, tryParseUrl } from "@/utils";
import type { ProjectQuery } from "..";
import { BrowserIcon, MobileDeviceIcon, OSIcon } from "../icons";

function formatRelativeTime(dateStr: string): string {
	try {
		const diff = Math.max(0, (Date.now() - new Date(dateStr).getTime()) / 1000);
		if (diff < 60) return "just now";
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		return `${Math.floor(diff / 86400)}d ago`;
	} catch {
		return dateStr;
	}
}

export const SessionsCard = ({ query }: { query: ProjectQuery }) => {
	const [limit, setLimit] = useState(50);
	const [expandedId, setExpandedId] = useState<string | null>(null);
	const { timeFormat } = useTimeFormat();

	const { sessions, isLoading, refetch } = useProjectSessions({
		projectId: query.project.id,
		range: query.range,
		limit,
	});

	const toggleExpand = (id: string) => {
		setExpandedId((prev) => (prev === id ? null : id));
	};

	return (
		<article className={cls("card", styles.card)}>
			<div className={styles.header}>
				<div className={styles.titleArea}>
					<UserIcon size={18} />
					<h3 className={styles.title}>Visitor Sessions</h3>
					<span className={styles.countBadge}>
						{sessions.length} {sessions.length === 1 ? "visitor" : "visitors"}
					</span>
				</div>
				<div className={styles.controls}>
					<select
						className={styles.limitSelect}
						value={limit}
						onChange={(e) => setLimit(Number(e.target.value))}
						aria-label="Sessions limit"
					>
						<option value={25}>25 sessions</option>
						<option value={50}>50 sessions</option>
						<option value={100}>100 sessions</option>
						<option value={200}>200 sessions</option>
					</select>
					<button
						type="button"
						className={styles.refreshButton}
						onClick={() => refetch()}
						title="Refresh sessions"
						aria-label="Refresh sessions"
					>
						<RotateCwIcon size={15} />
					</button>
				</div>
			</div>

			{isLoading && sessions.length === 0 ? (
				<div className={styles.loadingSpinner}>
					<div className="loading-spinner" />
				</div>
			) : sessions.length === 0 ? (
				<div className={styles.emptyState}>No visitor sessions found in this date range.</div>
			) : (
				<div className={styles.sessionsList}>
					{sessions.map((session) => (
						<SessionItem
							key={session.visitorGroupId}
							session={session}
							isExpanded={expandedId === session.visitorGroupId}
							onToggle={() => toggleExpand(session.visitorGroupId)}
							query={query}
							timeFormat={timeFormat}
						/>
					))}
				</div>
			)}
		</article>
	);
};

const SessionItem = ({
	session,
	isExpanded,
	onToggle,
	query,
	timeFormat,
}: {
	session: SessionRow;
	isExpanded: boolean;
	onToggle: () => void;
	query: ProjectQuery;
	timeFormat: TimeFormat;
}) => {
	const shortId = session.visitorGroupId.slice(0, 8);
	const flag = session.country ? countryCodeToFlag(session.country) : null;
	const locationLabel = [session.city, session.country].filter(Boolean).join(", ") || "Unknown Location";

	return (
		<div className={styles.sessionItem} data-expanded={isExpanded}>
			<div
				className={styles.sessionSummary}
				onClick={onToggle}
				onKeyDown={(e) => {
					if (e.key === "Enter" || e.key === " ") {
						e.preventDefault();
						onToggle();
					}
				}}
				role="button"
				tabIndex={0}
				aria-expanded={isExpanded}
			>
				<div className={styles.visitorCol} title={`Visitor ID: ${session.visitorGroupId}`}>
					<div className={styles.visitorAvatar}>
						<UserIcon size={12} />
					</div>
					<span>{shortId}</span>
				</div>

				<div className={styles.geoCol} title={locationLabel}>
					{flag ? <span className={styles.flag}>{flag}</span> : <GlobeIcon size={14} />}
					<span className={styles.locationText}>{locationLabel}</span>
				</div>

				<div className={styles.techCol}>
					<div className={styles.techIcons}>
						{session.browser && <BrowserIcon browser={session.browser} size={15} />}
						{session.platform && <OSIcon os={session.platform} size={15} />}
						{session.mobile !== null && <MobileDeviceIcon isMobile={session.mobile} size={15} />}
					</div>
					<span>{[session.browser, session.platform].filter(Boolean).join(" · ") || "Unknown"}</span>
				</div>

				<div className={styles.activityCol}>
					<span className={styles.badge} title={`${session.views} views, ${session.visits} visits`}>
						{session.views} {session.views === 1 ? "view" : "views"}
					</span>
					{session.visits > 1 && <span className={styles.badge}>{session.visits} visits</span>}
				</div>

				<div
					className={styles.timeCol}
					title={`Last active: ${new Date(session.lastSeen).toLocaleString([], { hour12: timeFormat === "12h" })}`}
				>
					{formatRelativeTime(session.lastSeen)}
				</div>

				<div className={styles.expandCol}>
					{isExpanded ? <ChevronUpIcon size={16} /> : <ChevronDownIcon size={16} />}
				</div>
			</div>

			{isExpanded && (
				<SessionTimeline
					projectId={query.project.id}
					visitorGroupId={session.visitorGroupId}
					query={query}
					timeFormat={timeFormat}
				/>
			)}
		</div>
	);
};

const SessionTimeline = ({
	projectId,
	visitorGroupId,
	query,
	timeFormat,
}: {
	projectId: string;
	visitorGroupId: string;
	query: ProjectQuery;
	timeFormat: TimeFormat;
}) => {
	const { timeline, isLoading } = useSessionTimeline({
		projectId,
		visitorGroupId,
		range: query.range,
	});

	if (isLoading && timeline.length === 0) {
		return (
			<div className={styles.timelineDrawer}>
				<div className={styles.loadingSpinner}>
					<div className="loading-spinner" />
				</div>
			</div>
		);
	}

	if (timeline.length === 0) {
		return (
			<div className={styles.timelineDrawer}>
				<div className={styles.emptyState}>No activity details found for this session.</div>
			</div>
		);
	}

	return (
		<div className={styles.timelineDrawer}>
			<div className={styles.timelineHeader}>Activity Timeline ({timeline.length} events)</div>
			<div className={styles.timelineList}>
				{timeline.map((event, idx) => (
					<TimelineItem key={`${event.createdAt}-${idx}`} event={event} timeFormat={timeFormat} />
				))}
			</div>
		</div>
	);
};

const TimelineItem = ({ event, timeFormat }: { event: SessionEvent; timeFormat: TimeFormat }) => {
	const time = formatEventTime(event.createdAt, timeFormat);
	const rawPath = event.path || "/";
	const urlObj = tryParseUrl(rawPath);
	const displayPath = typeof urlObj === "string" ? rawPath : formatPath(urlObj);
	const href = event.fqdn
		? `https://${event.fqdn}${displayPath.startsWith("/") ? displayPath : `/${displayPath}`}`
		: displayPath;

	return (
		<div className={styles.timelineItem}>
			<span className={styles.eventTime}>{time}</span>
			{event.event !== "pageview" && <span className={styles.eventBadge}>{event.event}</span>}
			<a
				href={href}
				target="_blank"
				rel="noreferrer"
				className={styles.eventPath}
				title={event.fqdn ? `https://${event.fqdn}${displayPath}` : displayPath}
			>
				{displayPath}
			</a>
			{event.referrer && (
				<span className={styles.eventReferrer} title={`Referrer: ${event.referrer}`}>
					via {event.referrer}
				</span>
			)}
		</div>
	);
};
