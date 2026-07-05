import json, urllib.request

def get(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'probe'})
    return json.loads(urllib.request.urlopen(req, timeout=15).read())

# Test 1: search API
ms = get("https://huggingface.co/api/models?search=Instruct+GGUF&filter=text-generation&sort=downloads&direction=-1&limit=15")
print("=== top downloads ===")
for m in ms[:15]:
    name = m['id']
    dl = m.get('downloads', 0)
    likes = m.get('likes', 0)
    print(f"  {name:55}  dl={dl:>8}  likes={likes:>4}")

# Test 2: tree API for size
print("\n=== qwen2.5 1.5B tree ===")
t = get("https://huggingface.co/api/models/Qwen/Qwen2.5-1.5B-Instruct-GGUF/tree/main")
for s in t:
    n = s.get('path', '')
    if 'q4_k_m' in n.lower() and n.endswith('.gguf'):
        print(f"  {n}  size={s.get('size',0)//1024//1024}MB  oid={s.get('oid','')[:12]}")
        break