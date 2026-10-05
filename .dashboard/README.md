# ARCMiS autooptimise console

Localhost-only ops dashboard for the autooptimise coverage-sweep campaign.

- Stack: SvelteKit + Svelte 5 + TypeScript, adapter-node.
- Run: `npm run dev` (127.0.0.1) or `npm run build && npm run preview`.
- Poll: client polls `GET /api/status` every 10s (`POLL_MS` in `src/lib/config.ts`).
- Actions: `POST /api/actions/{start|stop|restart-engine|rescore|ledger-sync|purge-orphans}`.
- Engine restart reads `DASHBOARD_SUDO_PASSWORD` from `.env` (gitignored).
