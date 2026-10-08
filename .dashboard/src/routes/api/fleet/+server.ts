import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types.js';
import { fleet } from '$lib/server/monitor.js';

export const prerender = false;

// Fleet health: GPUs, units, heartbeats, launcher tails.
// Manual refresh (button), not polled: runs systemctl and nvidia-smi.
export const GET: RequestHandler = async () => {
	const state = await fleet();
	return json(state, { headers: { 'cache-control': 'no-store' } });
};
