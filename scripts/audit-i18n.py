"""Find hardcoded visible strings in .vue files.

Strategy: scan each .vue file's <template> and <script setup> blocks.
- Find Chinese characters -> definitely hardcoded UI text.
- Find English-only text inside: label/title/placeholder/ElMessage/ElNotification/Element Plus components.

For each hit, print file:line + the snippet. We'll dedupe by key.
"""
import re
from pathlib import Path
from collections import defaultdict

ROOT = Path(r"C:\Users\19114\.minimax-agent-cn\projects\admin-suite\src")

# Pattern A: Chinese text (any CJK character)
CJK = re.compile(r"[\u4e00-\u9fff]+")
# Pattern B: English text that's likely visible (heuristic: inside a string with UI-like words)
UI_LIKE = re.compile(
    r"""(?xi)
    (?:                          # either English sentence in quotes
        ['"]([A-Z][A-Za-z0-9 ,.!?'\-/&:()]{4,80})['"]
    )
    |
    (?:                          # or placeholder-style value
        placeholder\s*=\s*['"]([^'"]+)['"]
    )
    |
    (?:                          # or label=
        label\s*=\s*['"]([^'"]+)['"]
    )
    |
    (?:                          # or title=
        title\s*=\s*['"]([^'"]+)['"]
    )
    """
)

seen = defaultdict(set)

for f in sorted(ROOT.rglob("*.vue")):
    src = f.read_text(encoding="utf-8")
    lines = src.splitlines()
    for i, line in enumerate(lines, 1):
        # CJK
        if CJK.search(line):
            # Show the line cleaned of leading ws
            snippet = line.strip()
            if len(snippet) > 200:
                snippet = snippet[:200] + "..."
            print(f"  CJK  {f.relative_to(ROOT.parent)}:{i}  {snippet}")
            continue
        # UI-like English (rough)
        m = UI_LIKE.search(line)
        if m:
            for g in m.groups():
                if not g:
                    continue
                # Skip boring English (e.g. "user:read", "admin")
                if any((c in g) for c in " .!?&/:—"):
                    snippet = line.strip()
                    if len(snippet) > 200:
                        snippet = snippet[:200] + "..."
                    print(f"  ENG  {f.relative_to(ROOT.parent)}:{i}  {snippet}")
                    break

# Also scan component files
for f in sorted(ROOT.rglob("Sidebar*.vue")) + sorted(ROOT.rglob("HeaderBar.vue")) + sorted(ROOT.rglob("Breadcrumb.vue")) + sorted(ROOT.rglob("LanguageSwitcher.vue")) + sorted(ROOT.rglob("ThemeSwitcher.vue")):
    if not f.exists():
        continue
    src = f.read_text(encoding="utf-8")
    for i, line in enumerate(src.splitlines(), 1):
        if CJK.search(line):
            print(f"  CJK  {f.relative_to(ROOT.parent)}:{i}  {line.strip()[:200]}")