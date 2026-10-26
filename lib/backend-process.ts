import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";

export const backendState = {
    baseUrl: ""
};

const python = fileURLToPath(
	new URL(
		process.platform === "win32"
			? "../.venv/Scripts/python.exe"
			: "../.venv/bin/python",
		import.meta.url
	)
);

const server = fileURLToPath(new URL("../backend/server.py", import.meta.url));

export async function startBackend(): Promise<void> {
    const backend = spawn(python, [server], {
        stdio: ["pipe", "pipe", "pipe"],
        windowsHide: true,
        env: {
            ...process.env,
            PYTHONUNBUFFERED: "1",
        }
    });

    const rl = createInterface({
        input: backend.stdout
    });

    const stderr = createInterface({
        input: backend.stderr
    });

    const port = await new Promise<number>((resolve, reject) => {
        let settled = false;
        const timeout = setTimeout(() => {
            backend.kill();
            fail(new Error("Backend startup timed out"));
        }, 60000);

        function fail(error: Error) {
            if (settled) return;
            settled = true;
            clearTimeout(timeout);
            reject(error);
        }

        rl.on("line", (line: any) => {
            if (!line.startsWith("PORT=")) return;
            const port = Number(line.slice(5));
            if (!Number.isInteger(port) || port < 1 || port > 65535) {
                fail(new Error("Invalid port"));
                return;
            }

            if (settled) return;
            settled = true;
            clearTimeout(timeout);
            resolve(port);
        });

        backend.once("error", fail);
        backend.once("exit", (code: any) => {
            fail(new Error(`Backend exited with code ${code}`));
            process.exit(1);
        });
    });

    backendState.baseUrl = `http://127.0.0.1:${port}`;

    rl.on("line", (line: any) => {
        console.log(`[Python] ${line}`)
    });

    stderr.on("line", (line: any) => {
        console.log(`[Python] ${line}`);
    })

    console.log("Backend ready:", backendState.baseUrl);
}