# The page's own outside (web/outside.js) at a seed and an hour, for outside.rs's frames_are_the_pages:
# Math.random is the crate's Rng (splitmix64) while the view is made and opened, the clock is fixed for its
# first frame (the only one: shown() is true once). Needs a demo server (BURROW_DEMO=1) and Chrome.
#   python3 tests/web/outside.py http://localhost:7791/
import base64, json, os, subprocess, sys, tempfile

url = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))
look = os.path.join(here, "../../../../../web/look.mjs")

def expr(seed, h, m, duel):
    b = "{id:'b', name:'b', color:'#2fa087'}" if duel else "null"
    return f"""(async () => {{
  let s = {seed}n; const M64 = (1n<<64n)-1n;
  const rand = () => {{ s = (s + 0x9e3779b97f4a7c15n) & M64; let z = s; z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & M64; z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & M64; z ^= z >> 31n; return Number(z >> 11n) / 2**53; }};
  const keep = Math.random, gh = Date.prototype.getHours, gm = Date.prototype.getMinutes;
  Math.random = rand;
  const cv = document.createElement('canvas'); cv.width = 256; cv.height = 144;
  let n = 0;
  const ov = outsideView(cv, {{ shown: () => n++ < 1, wears: () => null }});
  ov.open({{id:'you', name:'you', seed:'you', color:'#d97757', human:true}}, {b}, null);
  Math.random = keep;
  Date.prototype.getHours = function () {{ return {h}; }}; Date.prototype.getMinutes = function () {{ return {m}; }};
  await new Promise((r) => setTimeout(r, 400));
  Date.prototype.getHours = gh; Date.prototype.getMinutes = gm;
  return cv.toDataURL();
}})()"""

shots = [("noon", 42, 12, 15, True), ("night", 42, 23, 30, True), ("dusk", 42, 19, 36, False)]
steps = [{"wait": 2500}] + [{"js": expr(*a[1:])} for a in shots]
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump(steps, f)
out = subprocess.run(["node", look, url, "1280", "800", f.name], capture_output=True, text=True).stdout
urls = [json.loads(l[4:]) for l in out.splitlines() if l.startswith("js:")]
for (name, *_), u in zip(shots, urls):
    with open(os.path.join(here, f"outside-{name}.png"), "wb") as png:
        png.write(base64.b64decode(u.split(",", 1)[1]))
    print("outside-" + name + ".png")
