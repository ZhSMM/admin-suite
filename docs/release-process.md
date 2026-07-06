# Release process

> **Important (v0.7.3+):** commits to `main` no longer trigger a release
> automatically. The release workflow runs only when you trigger it
> manually from the GitHub Actions UI (or `gh workflow run`). This
> prevents accidental releases while iterating on features.

## Quick recipe

```bash
# 1. Make sure everything is green
cd src-tauri
cargo check
cargo test --lib migrate
cd ..
npx vue-tsc --noEmit -p tsconfig.json

# 2. Bump version + tag
# In src-tauri/tauri.conf.json, edit `version` from "0.7.3" → "0.7.4".
& "E:\installation\Git\bin\git.exe" add src-tauri/tauri.conf.json
& "E:\installation\Git\bin\git.exe" commit -m "chore: bump 0.7.3 → 0.7.4"

# 3. Push. DO NOT tag — the workflow no longer triggers on tag pushes.
& "E:\installation\Git\bin\git.exe" push origin main

# 4. Trigger the release workflow manually
gh workflow run release.yml --ref main
```

Once the workflow finishes, watch the run via:

```bash
cd /path/to/admin-suite
& 'E:\installation\Python\python.exe' poll-release.py vX.Y.Z
```

or check the GitHub Actions tab → run #N → "Build Windows installer".

## Why manual-only?

We hit the issue where every `git push` was followed by a fresh
release pipeline — every commit, every fix-up. If something was
slightly off (the v0.7.2 build had a missing `</el-table>` that
made every CI run die at vite-build), we'd silently burn GitHub
Actions minutes and confuse users with broken release pages.

Manual dispatch means:

- You stage commits on `main`, review them, then trigger exactly
  one build.
- A failed build doesn't publish anything; you fix the code, push,
  trigger again — users only see the working one.
- Hot-fixes can ride the same commit + tag bump without
  interfering with the next feature in flight.

## What's in the build

The single Windows job produces:

| File                              | Use                                      |
|-----------------------------------|------------------------------------------|
| `Admin.Suite_X.Y.Z_x64-setup.exe` | NSIS installer. Always attached.        |
| `Admin.Suite_X.Y.Z_x64_en-US.msi` | MSI installer. Always attached (preferred for Group Policy / corporate deploys). |
| `Admin.Suite_X.Y.Z_x64-setup.exe.sig` | Tauri-updater signature. Only when `TAURI_SIGNING_PRIVATE_KEY` is configured. |
| `*.nsis.zip`                     | Tauri-updater bundle. Same condition.    |
| `latest.json`                     | Tauri-updater manifest. Same condition. |

Until you add `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
to repo Secrets, only the bare `.exe` / `.msi` ships. The in-app updater
will show "no update available" until then — expected, not a bug.

## Working without `gh` CLI

If `gh` is not on PATH, the workflow also accepts
`workflow_dispatch` from the Actions tab (Actions → Release →
"Run workflow" → choose branch + inputs → Run).

## Rebuilding an existing release

The workflow now exposes two `workflow_dispatch` inputs:

| Input           | Default | Effect                                                               |
|-----------------|---------|----------------------------------------------------------------------|
| `version`       | *(empty)* | Override the tag name. Pass `v0.7.4` to attach this build to that tag. |
| `create_release`| `true`  | When `false`, the workflow only builds and uploads artifacts — no GitHub Release is created or overwritten. Use for a clean rebuild without polluting the existing release page. |

Typical rebuild recipe:

```bash
# 1. (Optional) re-trigger on a fresh runner — no code changes needed
gh workflow run release.yml --ref main \
  -f version=v0.7.4 \
  -f create_release=false
```

Then download the bundles from the Actions run page (Artifacts →
`admin-suite-windows`). When you're happy with them, do a final
`create_release=true` run to actually publish.

## v0.7.4 changes to this workflow

- Added `workflow_dispatch` inputs (`version`, `create_release`) so
  the same workflow can rebuild without re-tagging.
- `tauri.conf.json` bundles now include both `nsis` and `msi`.

## Self-elevating checklist

1. **Did you bump `tauri.conf.json`'s `version`?** The build reads
   it; if you skip this, the publish step is `Admin.Suite_0.7.3_x64-setup.exe`
   even when the source is 0.7.4.
2. **`cargo check` + `cargo test migrate` + `vue-tsc --noEmit`** all
   return zero? (Same green-light as PR CI.)
3. **Have you actually run the new code?** A passing build isn't
   the same as a working app — at least click through the changed
   page in `pnpm dev` / `npm run tauri dev`.
