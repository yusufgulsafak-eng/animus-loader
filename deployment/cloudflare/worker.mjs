// ANIMUS API public entrypoint: https://animus-api.intelpol.workers.dev
// Configure UPSTREAM_ORIGIN as a Cloudflare Worker SECRET (HTTPS origin URL).
// This Worker is a streaming REVERSE PROXY, not a redirect.
// IMPORTANT: It does not secure a publicly reachable origin. Apply separate
// origin access controls if you need to prevent bypassing the Worker.

const PUBLIC_HOST = "animus-api.intelpol.workers.dev";

function upstreamBase(env) {
  const value = String(env.UPSTREAM_ORIGIN || "").trim();
  if (!value) return null;
  let url;
  try { url = new URL(value); } catch { return null; }
  if (url.protocol !== "https:" || !url.hostname || url.hostname === PUBLIC_HOST || url.username || url.password || url.search || url.hash || url.pathname !== "/") return null;
  return url;
}
function rewriteLocation(location, origin, publicUrl) {
  if (!location) return location;
  try {
    const dest = new URL(location, origin);
    if (dest.origin === origin.origin) {
      dest.protocol = publicUrl.protocol;
      dest.host = publicUrl.host;
      return dest.toString();
    }
  } catch { /* leave malformed/relative redirects unchanged */ }
  return location;
}
function rewriteCookies(headers) {
  // Do not send a cookie with Domain=private-origin; host-only cookie binds to Worker.
  const cookies = typeof headers.getSetCookie === "function" ? headers.getSetCookie() : (headers.get("set-cookie") ? [headers.get("set-cookie")] : []);
  if (!cookies.length) return;
  headers.delete("set-cookie");
  for (const cookie of cookies) headers.append("set-cookie", cookie.replace(/;\s*domain=[^;]*/ig, ""));
}
export default {
  async fetch(request, env) {
    const incomingUrl = new URL(request.url);
    if (incomingUrl.hostname !== PUBLIC_HOST) return new Response("Unknown host", { status: 421 });
    const origin = upstreamBase(env);
    if (!origin) return new Response("Upstream is not configured", { status: 503 });

    // Preserve the exact path, query, HTTP method, body (streaming), and Range.
    const upstreamUrl = new URL(incomingUrl.pathname + incomingUrl.search, origin);
    const headers = new Headers(request.headers);
    for (const name of ["host", "x-forwarded-host", "x-forwarded-proto", "forwarded", "x-real-ip", "cf-connecting-ip", "cf-visitor"]) headers.delete(name);
    const upstreamRequest = new Request(upstreamUrl, {
      method: request.method,
      headers,
      body: (request.method === "GET" || request.method === "HEAD") ? undefined : request.body,
      redirect: "manual",
      duplex: "half",
    });
    let upstreamResponse;
    try { upstreamResponse = await fetch(upstreamRequest, { redirect: "manual" }); }
    catch { return new Response("API temporarily unavailable", { status: 502 }); }

    const responseHeaders = new Headers(upstreamResponse.headers);
    responseHeaders.delete("x-powered-by");
    responseHeaders.delete("x-backend-host");
    responseHeaders.delete("x-origin-host");
    const location = responseHeaders.get("location");
    if (location) responseHeaders.set("location", rewriteLocation(location, origin, incomingUrl));
    rewriteCookies(responseHeaders);
    return new Response(upstreamResponse.body, {
      status: upstreamResponse.status,
      statusText: upstreamResponse.statusText,
      headers: responseHeaders,
    });
  },
};
