<script lang="ts">
    import { backendState } from "./lib/backend-process.ts";
	import axios from "axios";
	import { onMount } from "svelte";

	let count = $state(0);
	let url = backendState.baseUrl;
	let response = $state(null);

	onMount(async () => {
		try {
			const sentence = "The unanimous Declaration of the thirteen united States of America, When in the Course of human events, it becomes necessary for one people to dissolve the political bands which have connected them with another, and to assume among the powers of the earth, the separate and equal station to which the Laws of Nature and of Nature's God entitle them, a decent respect to the opinions of mankind requires that they should declare the causes which impel them to the separation.";
			const words = sentence.match(/\b\w+\b/g);

			response = await axios.post(url + "/topics", {
				documents: words
			});
		} catch (err) {
			console.error(err);
		}
	});
</script>

<div class="app">
	<div class="card">
		<div class="hint">{url}</div>
	</div>
</div>

<style>
	.app {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
		background-color: #11111b;
	}
	.card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		padding: 32px;
		background-color: #1e1e2e;
		border-radius: 12px;
	}
	.hint {
		color: #a6adc8;
		font-size: 13px;
	}
</style>
