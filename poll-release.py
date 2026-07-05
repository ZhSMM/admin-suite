#!/usr/bin/env python3
"""Poll GitHub Actions for the v0.5.7 release build + release artefact."""
import json, sys, urllib.error, urllib.request


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "poll-release", "Accept": "application/vnd.github+json"})
    return json.loads(urllib.request.urlopen(req, timeout=15).read())


def workflow_done(tag):
    runs = get("https://api.github.com/repos/ZhSMM/admin-suite/actions/runs?per_page=20")
    cands = [r for r in runs["workflow_runs"] if r["name"] == "Release" and r.get("head_branch") == tag]
    if not cands:
        return False, f"no Release workflow run yet for tag {tag}"
    run = cands[0]
    if run["status"] != "completed":
        return False, f"Release run {run['id']} status={run['status']} (still building)"
    if run["conclusion"] != "success":
        return False, f"Release run {run['id']} conclusion={run['conclusion']}"
    return True, f"Release run {run['id']} succeeded"


def release_published(tag):
    rel = get(f"https://api.github.com/repos/ZhSMM/admin-suite/releases/tags/{tag}")
    if rel.get("draft"):
        return False, "release is still a draft"
    if rel.get("prerelease"):
        return False, "release marked as prerelease"
    assets = rel.get("assets", [])
    updater_assets = [a for a in assets if a["name"].lower().endswith((".msi.zip", ".nsis.zip"))]
    installer_assets = [a for a in assets if a["name"].lower().endswith((".msi", ".exe", ".appimage", ".deb"))]
    if not installer_assets:
        return False, f"no installer assets attached yet ({len(assets)} total assets)"
    if not updater_assets:
        return False, f"no updater assets attached (latest.json / *.zip missing) — auto-update won't work"
    names = ", ".join(a["name"] for a in installer_assets + updater_assets)
    return True, f"published with {len(installer_assets)} installer(s) + {len(updater_assets)} updater bundle(s): {names}"


def main():
    tag = sys.argv[1] if len(sys.argv) > 1 else "v0.5.7"
    ok, why = workflow_done(tag)
    if not ok:
        print(why); return 1
    ok, why = release_published(tag)
    if not ok:
        print(why); return 1
    print(f"DONE: {tag} {why}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except urllib.error.HTTPError as e:
        print(f"HTTP {e.code}: {e.reason}"); sys.exit(2)
    except Exception as e:
        print(f"error: {e}"); sys.exit(2)