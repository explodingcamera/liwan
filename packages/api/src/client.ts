export type EventMetadata = {
	url: string;
	referrer?: string;
	createdAt?: string | Date;
	userAgent?: string;
	ip?: string;
	screenWidth?: string;
	orientation?: string;
};

type QueuedEvent = Omit<EventMetadata, "createdAt"> & { entityId: string; name: string; createdAt: string };

export type ClientOptions = {
	/** Liwan base URL or the complete `/api/v1/events` endpoint. */
	endpoint: string;
	/** Liwan API key. */
	apiKey: string;
	batchSize?: number;
	flushInterval?: number;
	queueCapacity?: number;
	requestTimeout?: number;
	/** Retries after the first attempt, defaults to 4. */
	maxRetries?: number;
	fetch?: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
};

export type Client = {
	/** Queues an event for an entity without waiting for delivery. Throws if the queue is full or the client is closed. */
	event(entityId: string, name: string, metadata: EventMetadata): void;
	/** Waits for queued events to be delivered, or rejects on delivery failure. */
	flush(): Promise<void>;
	/** Stops accepting events and flushes the queue. Call this on shutdown. */
	close(): Promise<void>;
};

const sleep = (milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds));

const batchEndpoint = (endpoint: string) => {
	const url = new URL(endpoint);
	const path = url.pathname.replace(/\/$/, "");
	if (!path.endsWith("/api/v1/events")) {
		url.pathname = `${path}/api/v1/events`;
	}
	url.search = "";
	url.hash = "";
	return url.toString();
};

/** Creates a buffered server-side analytics client. */
export function createClient(options: ClientOptions): Client {
	const batchSize = options.batchSize ?? 100;
	const flushInterval = options.flushInterval ?? 5_000;
	const queueCapacity = options.queueCapacity ?? 10_000;
	const requestTimeout = options.requestTimeout ?? 10_000;
	const maxRetries = options.maxRetries ?? 4;
	if (!Number.isInteger(batchSize) || batchSize < 1 || batchSize > 10_000) {
		throw new Error("batchSize must be an integer between 1 and 10,000");
	}
	if (!Number.isFinite(flushInterval) || flushInterval < 1) throw new Error("flushInterval must be positive");
	if (!Number.isInteger(queueCapacity) || queueCapacity < batchSize) {
		throw new Error("queueCapacity must be an integer at least as large as batchSize");
	}
	if (!Number.isFinite(requestTimeout) || requestTimeout < 1) throw new Error("requestTimeout must be positive");
	if (!Number.isInteger(maxRetries) || maxRetries < 0 || maxRetries > 0xffffffff) {
		throw new Error("maxRetries must be a non-negative 32-bit integer");
	}
	if (!options.apiKey.trim()) throw new Error("apiKey is required");

	const fetchImpl = options.fetch ?? globalThis.fetch;
	if (!fetchImpl) throw new Error("fetch is required");
	const endpoint = batchEndpoint(options.endpoint);
	const queue: QueuedEvent[] = [];
	let queuedCount = 0;
	let closed = false;
	let timer: ReturnType<typeof setTimeout> | undefined;
	let activeFlush: Promise<void> | undefined;
	let lastError: Error | undefined;

	const send = async (entityId: string, events: Omit<QueuedEvent, "entityId">[]) => {
		for (let attempt = 0; attempt <= maxRetries; attempt++) {
			let response: Response;
			const controller = new AbortController();
			const timeout = setTimeout(() => controller.abort(), requestTimeout);
			try {
				response = await fetchImpl(endpoint, {
					method: "POST",
					headers: {
						Authorization: `Bearer ${options.apiKey}`,
						"Content-Type": "application/json",
					},
					body: JSON.stringify({ entityId, events }),
					signal: controller.signal,
				});
			} catch {
				if (attempt >= maxRetries) throw new Error("Liwan batch delivery failed after retries");
				const delay = 100 * 2 ** attempt;
				await sleep(delay + Math.random() * delay * 0.25);
				continue;
			} finally {
				clearTimeout(timeout);
			}

			if (response.ok) return;
			if (response.status === 401 || response.status === 400 || response.status === 413) {
				throw new Error(`Liwan rejected the batch with status ${response.status}`);
			}
			if (response.status !== 429 && response.status !== 503) {
				throw new Error(`Liwan returned status ${response.status}`);
			}
			if (attempt >= maxRetries) throw new Error(`Liwan batch delivery failed with status ${response.status}`);
			const retryAfter = Number(response.headers.get("Retry-After"));
			if (Number.isFinite(retryAfter) && retryAfter > 0) {
				await sleep(retryAfter * 1_000);
			} else {
				const delay = 100 * 2 ** attempt;
				await sleep(delay + Math.random() * delay * 0.25);
			}
		}
	};

	const startFlush = () => {
		if (timer) clearTimeout(timer);
		timer = undefined;
		if (activeFlush) return activeFlush;
		activeFlush = (async () => {
			while (queue.length > 0) {
				const entityId = queue[0].entityId;
				let count = 1;
				while (count < batchSize && count < queue.length && queue[count].entityId === entityId) count++;
				const events = queue.splice(0, count).map(({ entityId: _, ...event }) => event);
				try {
					await send(entityId, events);
				} catch (error) {
					lastError = error instanceof Error ? error : new Error("Liwan batch delivery failed");
					break;
				} finally {
					queuedCount -= events.length;
				}
			}
		})().finally(() => {
			activeFlush = undefined;
			if (!closed && queue.length > 0 && !timer) {
				timer = setTimeout(() => void startFlush(), flushInterval);
			}
		});
		return activeFlush;
	};

	const flush = async () => {
		await (activeFlush ?? startFlush());
		if (lastError) {
			const error = lastError;
			lastError = undefined;
			throw error;
		}
	};

	return {
		event(entityId, name, metadata) {
			if (closed) throw new Error("Liwan client is closed");
			if (!entityId.trim()) throw new Error("entityId is required");
			if (!name.trim()) throw new Error("event name is required");
			if (queuedCount >= queueCapacity) throw new Error("Liwan event queue is full");
			queue.push({
				...metadata,
				entityId,
				name,
				createdAt:
					metadata.createdAt instanceof Date
						? metadata.createdAt.toISOString()
						: (metadata.createdAt ?? new Date().toISOString()),
			});
			queuedCount++;
			if (queue.length >= batchSize) {
				void startFlush();
			} else if (!timer) {
				timer = setTimeout(() => void startFlush(), flushInterval);
			}
		},
		flush,
		async close() {
			closed = true;
			if (timer) clearTimeout(timer);
			timer = undefined;
			await flush();
		},
	};
}
