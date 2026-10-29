# The page's own sounds (web/sound.js) rendered offline in headless Chrome, nothing played: sound.js is loaded
# afresh for each sound with its AudioContext an OfflineAudioContext (48 kHz), the sound played at pan 0, the
# buffer's loudness every 10 ms written to sound-rms.json for lib.rs's sounds_are_the_pages.
#   python3 tests/web/sound.py http://localhost:7791/
import json, os, subprocess, sys, tempfile

url = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))
look = os.path.join(here, "../../../../../web/look.mjs")
names = ["ask", "beam", "door", "booth", "poof", "back", "shot", "clank", "flash", "bark", "clack", "print", "whoosh",
         "tune", "click", "paper", "cannon", "whistle", "blast", "wreck", "thud", "win", "died", "lose"]
js = """(async () => {
  const src = await (await fetch('/sound.js')).text(), out = {};
  Object.defineProperty(navigator, 'userActivation', { value: undefined, configurable: true });
  const keep = window.AudioContext;
  for (const name of %s) {
    let oc = null;
    window.AudioContext = function () { oc = new OfflineAudioContext(2, 48000 * 5, 48000); return oc; };
    (0, eval)(src);
    Sound.play(name, 0, 1, true, { dur: 2.5 });
    const b = await oc.startRendering(), d = b.getChannelData(0), w = 480, rms = [];
    for (let i = 0; i + w <= d.length; i += w) { let s = 0; for (let k = i; k < i + w; k++) s += d[k] * d[k]; rms.push(Math.sqrt(s / w)); }
    while (rms.length && rms[rms.length - 1] < 1e-6) rms.pop();
    out[name] = rms;
  }
  window.AudioContext = keep;
  return out;
})()""" % json.dumps(names)
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump([{"wait": 1500}, {"js": js}], f)
out = subprocess.run(["node", look, url, "800", "600", f.name], capture_output=True, text=True).stdout
res = [json.loads(l[4:]) for l in out.splitlines() if l.startswith("js:")][0]
with open(os.path.join(here, "sound-rms.json"), "w") as o:
    json.dump({k: [round(v, 6) for v in res[k]] for k in names}, o)
print({k: len(v) for k, v in res.items()})
