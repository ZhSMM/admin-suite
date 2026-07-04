"""Generate zh-TW.json from zh-CN.ts bundle.

Reads the bundled zh-CN export default object, converts every string value
from Simplified to Traditional Chinese via opencc's s2tw config, and writes
a JSON file matching the Locales import format:
  { "id": "zh-TW", "label": "...", "messages": { ... } }
"""
import json
import re
from pathlib import Path

import opencc

ROOT = Path(r"C:\Users\19114\.minimax-agent-cn\projects\admin-suite")
zh_cn_path = ROOT / "src" / "i18n" / "locales" / "zh-CN.ts"
out_path = ROOT / "dist" / "zh-TW.json"
out_path.parent.mkdir(exist_ok=True)

src = zh_cn_path.read_text(encoding="utf-8")

# Match each `'key': 'value'` line.  We deliberately don't try to run a JS
# parser — the bundle is a flat object literal so a regex is enough and
# avoids the npm dependency.
pattern = re.compile(r"'([^']+)':\s*'((?:\\'|[^'])*)'")

# Extract only the default-export object body (between the outer `{` and the
# matching `}`).  Cheap brace counter; the bundle has no nested objects.
body_start = src.find("export default {")
body = src[body_start + len("export default {"):]
depth = 0
end = 0
for i, ch in enumerate(body):
    if ch == "{":
        depth += 1
    elif ch == "}":
        if depth == 0:
            end = i
            break
        depth -= 1
inner = body[:end]

# Convert with opencc.  s2tw maps simplified -> traditional used in Taiwan.
converter = opencc.OpenCC("s2tw")

# Values that must NOT be translated (technical codes / placeholders):
SKIP_VALUES = {
    # Numbers, percentages, ranges — keep as-is.
}

def should_skip_value(value: str) -> bool:
    # Skip pure ASCII identifiers / format tokens — they would mangle if
    # transliterated, and they shouldn't be translated anyway.
    if value.startswith(("YYYY", "MM", "DD", "HH", "mm", "ss", "{", "$")):
        return True
    # Pure placeholder / pattern strings.
    if re.fullmatch(r"[A-Za-z0-9 .,;:!?/\\:\-_=+&@#$%^*()<>\[\]{}'\"`]+", value):
        return True
    return False

messages: dict[str, str] = {}
for m in pattern.finditer(inner):
    key, value = m.group(1), m.group(2)
    # Un-escape JS string.
    value = value.replace("\\'", "'").replace("\\\\", "\\")
    if should_skip_value(value):
        messages[key] = value
    else:
        messages[key] = converter.convert(value)

# Top-level shape matching Locales import contract.
payload = {
    "id": "zh-TW",
    "label": "繁體中文",
    "messages": messages,
}

out_path.write_text(
    json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8"
)
print(f"Wrote {out_path} with {len(messages)} keys")
# Spot-check a few keys
for k in ("common.ok", "menu.users", "tools.crypto.title", "settings.title"):
    print(f"  {k:30} -> {messages.get(k)}")