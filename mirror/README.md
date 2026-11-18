# Voice mirror

The apps download voices from here first, and fall back to the original
installers on archive.org. It is a Cloudflare R2 bucket (`quoquendo-voices`)
behind a Worker (`mirror/worker`), filled by the `mirror` GitHub Actions
workflow, which downloads all 47 installers from archive.org, unpacks them with
`quoquendo --unpack` and uploads the LTTS7 tree.

## URL layout

| URL | What |
|---|---|
| `<mirror>/v1/catalog.json` | every voice and language folder with each file's path, size and sha256 (`quoquendo --mirror-catalog`) |
| `<mirror>/v1/<path>` | one file of the LTTS7 tree, e.g. `v1/data/Roberto.vcf`, `v1/data/voices/Roberto/Roberto.sde`, `v1/bin/LTTS7Italian.dll` |
| `<mirror>/v1/archive/<installer>.exe` | the original installer, proxied from archive.org (which sends no CORS header for `.exe` files) |

Every response carries `Access-Control-Allow-Origin: *` and supports `Range`
requests (206). Tree files are cached for a year (they never change under a
path); `catalog.json` for 5 minutes.

## One-time setup

1. Enable R2 in the Cloudflare dashboard. Cloudflare requires a card on file;
   the free tier is 10 GB, and the voices take about 2.1 GB.
2. Create the bucket: `npx wrangler r2 bucket create quoquendo-voices`.
3. Create a Cloudflare API token with *Workers R2 Storage: Edit* and
   *Workers Scripts: Edit*, and add the GitHub repository secrets
   `CLOUDFLARE_API_TOKEN` (the token) and `CLOUDFLARE_ACCOUNT_ID`.
4. Run the **mirror** workflow (Actions → mirror → Run workflow).
5. Optionally add a custom domain to the `quoquendo-voices` Worker.
6. Set the repository variable `QQ_MIRROR_URL` to the Worker URL
   (`https://quoquendo-voices.<account>.workers.dev` or the custom domain).
   The Pages build and the apps read it at build time.

## Trying it locally

```sh
cd mirror/worker
npx wrangler@3 r2 object put quoquendo-voices/v1/catalog.json --file /tmp/qq-catalog.json --local
npx wrangler@3 dev   # http://127.0.0.1:8787/v1/catalog.json
```
