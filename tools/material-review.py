"""Make a local before/after viewer for material, model and sprite captures."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("captures", type=Path)
    parser.add_argument("--recipe", type=Path)
    args = parser.parse_args()
    views = json.loads((args.captures / "views.json").read_text())
    recipe = json.loads(args.recipe.read_text()) if args.recipe else {}
    for view in views:
        if "texture" in view:
            view["name"] = recipe.get(view["texture"], {}).get("name", view["texture"][:8])
            view["stem"] = view["texture"]
            view["details"] = "Texture " + view["texture"]
            if "sprite" in view:
                view["details"] += (f" · Sprite {view['sprite']} · left to right: "
                                    + ", ".join(view["blends_left_to_right"]))
            if not view.get("unobstructed", True):
                view["details"] += " · target partly obstructed"
        else:
            view["details"] = "Model " + view["model"] + " · " + view["pose"]
        for mode in ("original", "pack"):
            path = args.captures / f'{view["stem"]}-{view["view"]}-{mode}.png'
            if not path.is_file():
                parser.error(f"missing capture: {path}")
    data = json.dumps(views).replace("<", "\\u003c")
    page = """<!doctype html>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Material comparison</title>
<style>
body{margin:24px;background:#181b20;color:#e9e9ee;font:16px system-ui;max-width:1440px}
h1{font-size:24px}select,button{font:inherit;padding:7px;margin:0 12px 16px 0}
.frame{position:relative;aspect-ratio:16/9;background:#000;max-width:1280px}
.frame img{position:absolute;width:100%;height:100%;object-fit:contain}
#after{clip-path:inset(0 0 0 50%)}input{width:min(100%,1280px);margin-top:15px}
.labels{display:flex;justify-content:space-between;max-width:1280px}p{color:#b8bec9}
</style>
<h1>Material comparison</h1>
<p>Matching in-engine captures: same camera, lighting and simulation state.
The optional pack can replace textures and model geometry.</p>
<label>View <select id="view"></select></label><button id="original">Original</button>
<button id="split">Split</button><button id="pack">Replacement pack</button>
<div class="labels"><span>Original</span><span>Replacement pack</span></div>
<div class="frame"><img id="before" alt="Original textures"><img id="after" alt="Replacement textures"></div>
<input id="wipe" aria-label="Before and after divider" type="range" min="0" max="100" value="50">
<p id="details"></p>
<script>
const views=VIEWS;
const select=document.getElementById('view'),before=document.getElementById('before'),after=document.getElementById('after'),wipe=document.getElementById('wipe');
views.forEach((v,i)=>{const o=new Option(v.name.replaceAll('_',' ')+' / '+v.view,i);select.add(o)});
function update(){const v=views[select.value];const p=v.stem+'-'+v.view;before.src=p+'-original.png';after.src=p+'-pack.png';document.getElementById('details').textContent=v.details;}
function divide(n){wipe.value=n;after.style.clipPath='inset(0 0 0 '+n+'%)'}
select.onchange=update;wipe.oninput=()=>divide(wipe.value);
document.getElementById('original').onclick=()=>divide(100);
document.getElementById('split').onclick=()=>divide(50);
document.getElementById('pack').onclick=()=>divide(0);
select.value=Math.max(0,views.findIndex(v=>v.name==='roof'&&v.view==='angle'));update();
</script>
""".replace("VIEWS", data)
    output = args.captures / "index.html"
    output.write_text(page)
    print(output)


if __name__ == "__main__":
    main()
