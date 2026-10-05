import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types.js';
import { runAction, type ActionName } from '$lib/server/actions.js';

const ACTIONS = new Set<string>([
	'start',
	'stop',
	'restart-engine',
	'rescore',
	'ledger-sync',
	'purge-orphans'
]);

export const POST: RequestHandler = async ({ params, request }) => {
	const action = params.action;
	if (!ACTIONS.has(action)) error(404, 'unknown action');
	let body: { dir?: string; confirm?: boolean } = {};
	try {
		const raw: unknown = await request.json();
		if (raw && typeof raw === 'object') {
			if ('dir' in raw && typeof raw.dir === 'string') body.dir = raw.dir;
			if ('confirm' in raw && typeof raw.confirm === 'boolean') body.confirm = raw.confirm;
		}
	} catch {
		// empty body is fine for most actions
	}
	const result = await runAction(action as ActionName, body);
	return json(result, { status: result.ok ? 200 : 500, headers: { 'cache-control': 'no-store' } });
};
