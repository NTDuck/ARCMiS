import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types.js';
import { campaign } from '$lib/server/monitor.js';

export const prerender = false;

// Campaign state: verdict table, families, pair position, death census,
// admission-503 census, wall-time history. Cheap file reads only.
export const GET: RequestHandler = async () => {
	const state = await campaign();
	return json(state, { headers: { 'cache-control': 'no-store' } });
};
