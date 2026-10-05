import { execFile } from 'node:child_process';
import type { ExecFileException } from 'node:child_process';

export type RunResult = { ok: boolean; out: string; err: string };

function settle(
	error: ExecFileException | null,
	stdout: string | Buffer,
	stderr: string | Buffer
): RunResult {
	if (error) {
		return {
			ok: false,
			out: String(stdout ?? ''),
			err: String(stderr ?? '') || error.message
		};
	}
	return { ok: true, out: String(stdout ?? ''), err: String(stderr ?? '') };
}

/** Run a command with an arg array and a hard timeout. Never a shell string. */
export function run(
	file: string,
	args: string[],
	timeoutMs = 3000,
	cwd?: string
): Promise<RunResult> {
	return new Promise((resolve) => {
		execFile(
			file,
			args,
			{ timeout: timeoutMs, maxBuffer: 1024 * 1024, ...(cwd ? { cwd } : {}) },
			(error, stdout, stderr) => resolve(settle(error, stdout, stderr))
		);
	});
}

/** Run a command with a hard timeout, feeding `input` on stdin (sudo -S password path). */
export function runWithStdin(
	file: string,
	args: string[],
	input: string,
	timeoutMs = 3000
): Promise<RunResult> {
	return new Promise((resolve) => {
		const child = execFile(
			file,
			args,
			{ timeout: timeoutMs, maxBuffer: 1024 * 1024 },
			(error, stdout, stderr) => resolve(settle(error, stdout, stderr))
		);
		child.stdin?.end(input);
	});
}
