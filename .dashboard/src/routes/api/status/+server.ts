import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types.js';
import { scoreboard, units, heartbeats, inFlight, recentCloses, engine, journal } from '$lib/server/probes.js';
import type { StatusPayload } from '$lib/types.js';

export const prerender = false;

export const GET: RequestHandler = async () => {
	const [sb, un, hb, flying, closes, eng, jrnl] = await Promise.all([
		scoreboard(),
		units(),
		heartbeats(),
		inFlight(),
		recentCloses(),
		engine(),
		journal()
	]);
	const payload: StatusPayload = {
		now: new Date().toISOString(),
		scoreboard: sb,
		units: un,
		heartbeats: hb,
		inFlight: flying,
		recentCloses: closes,
		engine: eng,
		journal: jrnl
	};
	return json(payload, { headers: { 'cache-control': 'no-store' } });
};
