<script>
  let connected = null;

  async function checkStatus() {
    try {
      const res = await fetch('http://127.0.0.1:4901/status');
      const data = await res.json();
      connected = data.connected;
    } catch {
      connected = false;
    }
  }

  checkStatus();
  setInterval(checkStatus, 5000);
</script>

<main>
  <h1>Connection Status</h1>
  {#if connected === null}
    <p>Checking...</p>
  {:else if connected}
    <p style="color: green;">✓ Connected to 8.8.8.8</p>
  {:else}
    <p style="color: red;">✗ Not connected to 8.8.8.8</p>
  {/if}
</main>

<style>
  main {
    font-family: sans-serif;
    padding: 2rem;
  }
</style>
