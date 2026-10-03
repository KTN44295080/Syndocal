import { performance } from 'node:perf_hooks';

// Native admission may acknowledge an authenticated read before renderer
// completion. Settle only that receipt, without replaying discovery or caching
// authentication. The caller owns one admission slot throughout this sequence.
export async function settleAuthenticatedDiscovery(requestId, send) {
  let response = await send('control_plane.get_capabilities', {});
  const deadline = performance.now() + 3000;
  for (let lookups = 0; response.status === 'pending' && lookups < 4 && performance.now() < deadline; lookups++) {
    await new Promise(resolve => setTimeout(resolve, 100));
    response = await send('request.status', { requestId });
  }
  return response;
}
