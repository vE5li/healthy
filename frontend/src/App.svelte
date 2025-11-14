<script>
  let devices = [];

  // Use environment variable for backend URL, fallback to localhost for development
  const BACKEND_URL = import.meta.env.VITE_BACKEND_URL || 'http://127.0.0.1:4901';

  async function checkStatus() {
    try {
      const res = await fetch(`${BACKEND_URL}/status`);
      const data = await res.json();
      devices = data;
    } catch {
      devices = [];
    }
  }

  checkStatus();
  setInterval(checkStatus, 500);
</script>

<main>
  <h1>Device Status</h1>
  {#if devices.length === 0}
    <p>Loading...</p>
  {:else}
    <ul>
      {#each devices as device}
        <li>
          {#if device.connected}
            <span style="color: green;">✓ {device.ip}</span>
          {:else}
            <span style="color: red;">✗ {device.ip}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    font-family: sans-serif;
    padding: 2rem;
  }
  ul {
    list-style: none;
    padding: 0;
  }
  li {
    margin: 0.5rem 0;
  }
</style>
