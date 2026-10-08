import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types.js';
import { roundDetail, activeRoundDirs } from '$lib/server/monitor.js';

export const prerender = false;

// Live round detail: phase timeline, throughput, decisions, task graph.
// Cheap file reads only, safe to poll.
export const GET: RequestHandler = async ({ params }) => {
	const dir = params.dir;
	if (!/^[A-Za-z0-9_.-]+$/.test(dir) || dir.startsWith('.')) error(400, 'invalid dir name');
	const active = await activeRoundDirs();
	if (!active.includes(dir)) error(404, 'not an active round dir');
	const detail = await roundDetail(dir);
	return json(detail, { headers: { 'cache-control': 'no-store' } });
};
