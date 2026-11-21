# quoquendo on Cloudflare

One Worker (`mirror/worker`, named `quoquendo`) over one R2 bucket
(`quoquendo`) serves both the web app and the voices, from one origin:

| URL | What | Filled by |
|---|---|---|
| `/`, `/<file>` | the web app (`trunk build` output, R2 `site/`) | the **site** workflow, on every push to `main` |
| `/v1/catalog.json` | every voice and language folder with each file's path, size and sha256 (`quoquendo --mirror-catalog`) | the **mirror** workflow, run by hand |
| `/v1/<path>` | one file of the LTTS7 tree, e.g. `v1/data/Roberto.vcf`, `v1/bin/LTTS7Italian.dll` | the **mirror** workflow |
| `/v1/archive/<installer>.exe` | the original installer, proxied from archive.org (which sends no CORS header for `.exe` files) | — |

The web app downloads voices from the site it is served from. The desktop
and Android apps use the URL set in their settings, or `QQ_MIRROR_URL` given
at build time. Every `/v1/` response carries `Access-Control-Allow-Origin: *`
and supports `Range` requests. The voices' files and the app's hashed files
are cached for a year (they never change under a name); `catalog.json`,
`index.html` and the unpack worker's files for 5 minutes.

The mirror workflow downloads all 47 installers from archive.org (kept in
the Actions cache for the next run), and the Catalan & Valencian and
Galician language distributions from the bucket's `distributions/` (on no
public site, so put there by hand; the Worker doesn't serve that prefix):

```sh
npx wrangler@4 r2 object put "quoquendo/distributions/Galician Distribution.exe" --file "Galician Distribution.exe" --remote
```

It unpacks them with `quoquendo --unpack` and uploads the LTTS7 tree
(about 2.1 GB): only the files whose sha256 differs from the catalog
already on the mirror, then the catalog. Both workflows deploy the Worker.

## One-time setup

1. **R2.** In the Cloudflare dashboard, open *R2 Object Storage* and enable it
   (Cloudflare asks for a card; the free tier covers 10 GB stored and
   egress is free). Then *Create bucket*, name `quoquendo`, location
   *Automatic*.
2. **workers.dev subdomain.** Open *Workers & Pages* once: on a new account
   it asks you to pick your `<name>.workers.dev` subdomain. The site will be
   `https://quoquendo.<name>.workers.dev`.
3. **API token.** *My Profile → API Tokens → Create Token → Create Custom
   Token*, with these permissions on your account:
   - *Account → Workers R2 Storage → Edit*
   - *Account → Workers Scripts → Edit*
4. **Account ID.** On the *Workers & Pages* overview (right-hand column), or
   from `npx wrangler whoami`.
5. **GitHub.** In the repository's *Settings → Secrets and variables →
   Actions*, add the secrets `CLOUDFLARE_API_TOKEN` and
   `CLOUDFLARE_ACCOUNT_ID`.
6. **Fill it.** *Actions → mirror → Run workflow* (the first run uploads
   every file; later ones only what changed), and *Actions → site → Run
   workflow* (or push to `main`).
7. **Your own domain.** `routes` in `mirror/worker/wrangler.toml` attaches
   it on every deploy (now `quoquendo.depau.eu`); the domain must be a zone
   on the same Cloudflare account. Set the repository variable
   `QQ_MIRROR_URL` to it: the mirror workflow reads the live catalog there,
   and the Android nightly uses it as its built-in mirror.

The bucket and Worker names are `quoquendo` in `mirror/worker/wrangler.toml`
and `BUCKET` in both workflows; change all three to use another name.

## Trying it locally

```sh
trunk build --release --public-url /
cd mirror/worker
for f in ../../dist/*; do npx wrangler@3 r2 object put "quoquendo/site/${f##*/}" --file "$f" --local; done
npx wrangler@3 r2 object put quoquendo/v1/catalog.json --file /tmp/qq-catalog.json --local
npx wrangler@3 dev   # http://localhost:8787/
```
