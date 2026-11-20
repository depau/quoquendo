// quoquendo on Cloudflare, one Worker over one R2 bucket:
// - /v1/<key>: the voice mirror, R2 files under v1/;
// - /v1/archive/<file>.exe: a CORS proxy for the archive.org installers
//   (archive.org sends no Access-Control-Allow-Origin for .exe files);
// - everything else: the web app, R2 files under site/ (`trunk build`'s
//   dist), so the app and its voices share one origin.

const ARCHIVE = "https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/";

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Expose-Headers": "Content-Length, Content-Range, ETag",
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const method = request.method;
    if (method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
          "Access-Control-Allow-Headers": "Range, If-None-Match",
          "Access-Control-Max-Age": "86400",
        },
      });
    }
    if (method !== "GET" && method !== "HEAD") {
      return new Response("method not allowed", { status: 405, headers: { ...CORS, Allow: "GET, HEAD, OPTIONS" } });
    }
    if (url.pathname.startsWith("/v1/archive/")) return archive(request, url.pathname.slice("/v1/archive/".length));
    const inV1 = url.pathname.startsWith("/v1/");
    let key;
    try {
      key = decodeURIComponent(url.pathname.slice(1));
    } catch {
      return notFound();
    }
    if (!inV1) key = "site/" + (key === "" || key.endsWith("/") ? key + "index.html" : key);
    if (key === "v1/" || key.split("/").some((s) => s === ".." || s === "")) return notFound();
    // trunk names the app's files by their hash: those never change; the
    // catalog, index.html and the worker loader do
    const fixed = inV1 ? key !== "v1/catalog.json" : /-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$/.test(key);
    return object(request, env, key, fixed ? "public, max-age=31536000, immutable" : "public, max-age=300");
  },
};

function notFound() {
  return new Response("not found", { status: 404, headers: CORS });
}

// what `wrangler r2 object put` leaves unset
const TYPES = {
  html: "text/html; charset=utf-8",
  js: "text/javascript",
  wasm: "application/wasm",
  css: "text/css",
  json: "application/json",
  svg: "image/svg+xml",
  png: "image/png",
  ico: "image/x-icon",
};

async function object(request, env, key, cache) {
  const obj = await env.VOICES.get(key, { range: request.headers, onlyIf: request.headers });
  if (obj === null) return notFound();
  const headers = new Headers(CORS);
  obj.writeHttpMetadata(headers);
  headers.set("ETag", obj.httpEtag);
  headers.set("Accept-Ranges", "bytes");
  headers.set("Cache-Control", cache);
  if (!headers.has("Content-Type")) {
    const type = TYPES[key.slice(key.lastIndexOf(".") + 1).toLowerCase()];
    if (type) headers.set("Content-Type", type);
  }
  // a precondition that failed gives an object without a body
  if (!("body" in obj)) return new Response(null, { status: 304, headers });
  let status = 200;
  const r = obj.range;
  if (r && request.headers.has("Range")) {
    let offset, length;
    if ("suffix" in r) {
      length = Math.min(r.suffix, obj.size);
      offset = obj.size - length;
    } else {
      offset = r.offset ?? 0;
      length = r.length ?? obj.size - offset;
    }
    status = 206;
    headers.set("Content-Range", `bytes ${offset}-${offset + length - 1}/${obj.size}`);
    headers.set("Content-Length", String(length));
  } else {
    headers.set("Content-Length", String(obj.size));
  }
  return new Response(request.method === "HEAD" ? null : obj.body, { status, headers });
}

async function archive(request, file) {
  let name;
  try {
    name = decodeURIComponent(file);
  } catch {
    return notFound();
  }
  if (!name || name.includes("/") || name.includes("\\") || !/\.exe$/i.test(name)) return notFound();
  const fwd = new Headers();
  const range = request.headers.get("Range");
  if (range) fwd.set("Range", range);
  const upstream = await fetch(ARCHIVE + encodeURIComponent(name), {
    method: request.method,
    headers: fwd,
    redirect: "follow",
    cf: { cacheEverything: true, cacheTtl: 86400 },
  });
  const headers = new Headers(CORS);
  for (const h of ["Content-Length", "Content-Range", "Content-Type", "ETag", "Last-Modified", "Accept-Ranges"]) {
    const v = upstream.headers.get(h);
    if (v !== null) headers.set(h, v);
  }
  return new Response(request.method === "HEAD" ? null : upstream.body, { status: upstream.status, headers });
}
