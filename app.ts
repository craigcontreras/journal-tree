import { render_hot } from 'gpuix-svelte';
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const python = fileURLToPath(
	new URL(
		process.platform === "win32"
			? "./.venv/Scripts/python.exe"
			: "./.venv/bin/python",
		import.meta.url
	)
);

const server = fileURLToPath(new URL("./backend/server.py", import.meta.url));

const backend = spawn(python, [server], {
	stdio: ["pipe", "inherit", "inherit"],
	windowsHide: true,
	env: {
		...process.env,
		PYTHONUNBUFFERED: "1",
	}
});

backend.on("spawn", () => {
	console.log("Started backend");
});

backend.on("error", (error) => {
	console.error("Failed to launch the backend: ", error);
});

backend.unref();

function cleanup() {
	if (backend.exitCode === null) {
		backend.kill("SIGTERM");
	}
}

process.once("SIGINT", () => {
	cleanup();
});

process.once("SIGTERM", () => {
	cleanup();
});

render_hot(new URL('./App.svelte', import.meta.url), {
	title: 'gpuix-svelte starter',
	width: 640,
	height: 480,
});

//GPUIX-svelte doesn't call exit handlers!!!!