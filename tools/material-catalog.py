"""Merge texture exports into a searchable authoring catalogue with usage evidence."""

import argparse
import json
import shutil
from pathlib import Path

from PIL import Image, ImageDraw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exports", type=Path, nargs="+")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--recipe", type=Path)
    args = parser.parse_args()
    if args.output.exists() and any(args.output.iterdir()):
        parser.error("output must be empty; use a new directory to preserve review notes")
    recipe = json.loads(args.recipe.read_text()) if args.recipe else {}
    entries = {}
    missions = {}
    for directory in args.exports:
        catalog = json.loads((directory / "catalog.json").read_text())
        if "mission_usage" in catalog:
            missions[directory.name] = {"stage": catalog["stage"], **catalog["mission_usage"]}
        models = {m["id"]: m["label"] for m in catalog["models"]}
        for identity in catalog["textures"]:
            detail = catalog.get("texture_details", {}).get(identity)
            if detail is None:
                parser.error(f"{directory}: export again to include texture usage information")
            entry = entries.setdefault(identity, {
                "id": identity, "size": detail["size"], "alpha": detail["alpha"],
                "scrolling": False, "models": {}, "sprites": [], "terrain_attributes": [],
                "map_samples": [], "exports": [], "source": str(directory / "textures" / f"{identity}.png"),
            })
            entry["scrolling"] |= detail["scrolling"]
            entry["sprites"] = sorted(set(entry["sprites"] + detail.get("sprites", [])))
            entry["exports"].append(directory.name)
            entry["terrain_attributes"] = sorted(set(entry["terrain_attributes"] + detail["terrain_attributes"]))
            entry["map_samples"] = detail["map_samples"] or entry["map_samples"]
            for model, usage in detail["models"].items():
                merged = entry["models"].setdefault(model, {
                    **usage, "label": models[model], "labels": [], "variants": {},
                })
                merged["labels"] = sorted(set(merged["labels"] + [models[model]]))
                merged["variants"][directory.name] = {**usage, "label": models[model]}
                merged["triangles"] = max(merged["triangles"], usage["triangles"])
                merged["blends"] = sorted(set(merged["blends"] + usage["blends"]))
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "textures").mkdir()
    for entry in entries.values():
        categories = set()
        if entry["sprites"]:
            categories.add("sprites")
        for model in entry["models"]:
            if "/map/" in model:
                categories.add("scenery")
            elif "/objects/" in model:
                categories.add("props")
            elif model.startswith("actors/"):
                categories.add("actors")
            else:
                categories.add("equipment")
        entry["categories"] = sorted(categories)
        entry["triangles"] = sum(u["triangles"] for u in entry["models"].values())
        authored = recipe.get(entry["id"], {})
        entry["name"] = authored.get("name", "")
        entry["status"] = authored.get("review", "authored" if authored else "todo")
        shutil.copyfile(entry["source"], args.output / "textures" / f'{entry["id"]}.png')
    ordered = sorted(entries.values(), key=lambda e: (e["categories"], -e["triangles"], e["id"]))
    (args.output / "catalog.json").write_text(json.dumps({"version": 1, "textures": list(entries), "entries": ordered, "missions": missions}, indent=2) + "\n")
    for category in ("scenery", "props", "actors", "equipment", "sprites"):
        selected = [e for e in ordered if category in e["categories"]]
        for start in range(0, len(selected), 32):
            sheet = Image.new("RGB", (1024, 8 * 156), "#353535")
            draw = ImageDraw.Draw(sheet)
            for i, entry in enumerate(selected[start:start + 32]):
                x, y = (i % 4) * 256, (i // 4) * 156
                with Image.open(args.output / "textures" / f'{entry["id"]}.png') as source:
                    texture = source.convert("RGBA")
                    texture.thumbnail((120, 120), Image.Resampling.NEAREST)
                    scale = min(120 // texture.width, 120 // texture.height)
                    texture = texture.resize((texture.width * scale, texture.height * scale), Image.Resampling.NEAREST)
                    sheet.paste(texture, (x + 4, y + 4), texture)
                labels = sorted({label.split(", part")[0]
                                 for u in entry["models"].values() for label in u["labels"]
                                 if not label.startswith("Map part")})
                if entry["sprites"]:
                    labels.append("Sprites " + ",".join(map(str, entry["sprites"])))
                draw.text((x + 128, y + 8), str(start + i) + " " + entry["id"][:8], fill="white")
                draw.text((x + 128, y + 28), "x".join(map(str, entry["size"])), fill="#bbbbbb")
                for row, label in enumerate(labels[:4]):
                    draw.text((x + 128, y + 48 + row * 14), label[:19], fill="#dddddd")
                flags = ["SCROLL" if entry["scrolling"] else "", "WATER" if any(a & 4 for a in entry["terrain_attributes"]) else "", entry["status"]]
                draw.text((x + 4, y + 132), " ".join(filter(None, flags)), fill="#9ad4ed")
            sheet.save(args.output / f"{category}-{start // 32}.png")
    data = json.dumps(ordered).replace("<", "\\u003c")
    page = """<!doctype html><meta charset="utf-8"><title>Texture authoring catalogue</title>
<style>body{font:15px system-ui;background:#171a20;color:#ddd;margin:24px}input,select{font:inherit;padding:8px;margin:0 12px 16px 0}#grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(290px,1fr));gap:16px}.card{background:#292e38;padding:14px;border-radius:8px;overflow:hidden}.card img{width:128px;height:128px;object-fit:contain;image-rendering:pixelated;background:#454545}small{display:block;overflow-wrap:anywhere;color:#abb5c5}p{margin:7px 0}details{font-size:12px}.warning{color:#ffcc80}</style>
<h1>Texture authoring catalogue</h1><p>Usage evidence from the game. Authored does not mean visually approved.</p>
<input id="search" placeholder="Name, model, identity…"><select id="category"><option value="">All uses</option><option>scenery</option><option>props</option><option>actors</option><option>equipment</option><option>sprites</option></select><select id="status"><option value="">All statuses</option><option>todo</option><option>authored</option><option>reviewed</option></select><p id="count"></p><div id="grid"></div>
<script>const entries=DATA;const $=id=>document.getElementById(id);function render(){const q=$('search').value.toLowerCase(),cat=$('category').value,status=$('status').value;const selected=entries.filter(e=>(!cat||e.categories.includes(cat))&&(!status||e.status===status)&&JSON.stringify(e).toLowerCase().includes(q));$('count').textContent=selected.length+' / '+entries.length+' texture identities';$('grid').replaceChildren();for(const e of selected){const card=document.createElement('div');card.className='card';const img=document.createElement('img');img.loading='lazy';img.src='textures/'+e.id+'.png';img.alt=e.name||e.id;card.append(img);for(const [tag,text] of [['p',e.name||e.id.slice(0,8)],['small',e.id],['p',e.size.join(' × ')+' · '+e.categories.join(', ')+' · '+e.status],['small',e.triangles+' triangles · '+Object.keys(e.models).length+' models'],['p',(e.scrolling?'Animated scroll · ':'')+(e.terrain_attributes.some(a=>a&4)?'Touches water terrain':'')]]){const n=document.createElement(tag);n.textContent=text;card.append(n)}const details=document.createElement('details'),summary=document.createElement('summary'),pre=document.createElement('pre');summary.textContent='Usage and alpha';pre.textContent=JSON.stringify({alpha:e.alpha,sprites:e.sprites,terrain:e.terrain_attributes,models:e.models,exports:e.exports},null,2);details.append(summary,pre);card.append(details);$('grid').append(card)}}for(const id of ['search','category','status'])$(id).addEventListener('input',render);render();</script>""".replace("DATA", data)
    (args.output / "index.html").write_text(page)
    print(f"Saved {len(entries)} texture identities to {args.output}")


if __name__ == "__main__":
    main()
