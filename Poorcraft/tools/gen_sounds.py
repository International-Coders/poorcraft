#!/usr/bin/env python3
"""Generate the LOREFORGE sound bank via the ElevenLabs Sound Effects API.

Usage:
    export ELEVENLABS_API_KEY=sk_...
    python3 tools/gen_sounds.py [names...]     # no args = all missing

Writes MP3s to assets/sounds/. Existing non-empty files are NEVER
regenerated (deliberate: one API generation per event, no re-rolls —
the key is a free-tier account and spend stays conscious). The API key
is read from the environment only; it must never be committed.

The manifest below is the single source of truth for what each sound is
supposed to be; lf_audio embeds and decodes the files at build time.
"""

import json
import os
import sys
import time
import urllib.error
import urllib.request

API = "https://api.elevenlabs.io/v1/sound-generation"
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "sounds")

# (file name, prompt, duration_seconds, prompt_influence)
# Prompts describe ONE single event — no ambience beds, no music.
MANIFEST = [
    # -- block break, per material family --
    ("break_wood",   "a wooden block splitting apart, sharp dry crack with splinters, single impact, game sound effect", 1.5, 0.3),
    ("break_stone",  "a stone block breaking apart, gritty crack and rocky crumble, single impact, game sound effect", 1.5, 0.3),
    ("break_metal",  "metal breaking with a sharp metallic clang, wrenching steel, single impact, game sound effect", 1.5, 0.3),
    ("break_glass",  "glass shattering into bright shards, single burst, game sound effect", 1.5, 0.3),
    ("break_soft",   "a dirt block crushed apart, loud soil crumble impact, game sound effect", 1.5, 0.3),
    # -- block place --
    ("place_wood",   "a wooden block set down with a firm hollow knock, game sound effect", 1.0, 0.3),
    ("place_stone",  "a heavy stone block placed down with a solid stone thud, game sound effect", 1.0, 0.3),
    ("place_metal",  "a metal block clunked into place, metallic clank, game sound effect", 1.0, 0.3),
    ("place_glass",  "a glass block firmly placed down onto stone, sharp glass clink", 1.0, 0.3),
    ("place_soft",   "soft dirt patted into place, muffled earthy thump, game sound effect", 1.0, 0.3),
    # -- footsteps per material (texture-first phrasing: "boot stomp"
    # generations came back near-silent twice, impact textures did not) --
    ("step_wood",    "a single hard knuckle knock on a thick wooden board, resonant loud knock", 1.0, 0.3),
    ("step_stone",   "gravel and stones grinding underfoot, loud gritty crunch step", 1.0, 0.3),
    ("step_metal",   "a steel plate clanked by a boot heel, loud metallic clomp", 1.0, 0.3),
    ("step_glass",   "a single footstep on ice with a brittle crunch", 0.5, 0.3),
    ("step_soft",    "a wet grass squelch, loud squishy footstep on mud", 1.0, 0.3),
    # -- ui / body / feedback --
    ("ui_click",     "a crisp UI button click, sharp wooden tap, short and loud", 1.0, 0.3),
    ("eat",          "two quick bites of a crunchy apple, chewing, short", 2.0, 0.3),
    ("hurt",         "a short pained grunt with a dull body impact, game character hurt", 1.0, 0.3),
    ("xp",           "a bright magical sparkle chime, two quick ascending bell notes", 1.5, 0.3),
    # -- timber --
    ("tree_creak",   "a big tree creaking and groaning as it starts to lean and fall", 3.0, 0.3),
    ("tree_crash",   "a large tree falling and crashing to the ground, branches snapping, heavy thud", 4.0, 0.3),
    # -- combat --
    ("bow_shoot",    "a bow string twang with an arrow releasing, short whoosh", 1.5, 0.3),
    ("arrow_hit",    "an arrow thunking hard into a wooden target, deep sharp impact", 1.5, 0.3),
    ("melee_swing",  "a sword swung fast through air, single whoosh", 1.0, 0.3),
    ("mob_hit",      "a fleshy impact thud hitting a creature, single hit", 1.0, 0.3),
    ("mob_death",    "a creature collapsing, heavy body fall thump", 1.5, 0.3),
    ("dragon_roar",  "a mighty dragon roar, deep growling beast bellow", 5.0, 0.3),
    # -- world / items --
    ("splash",       "a water splash, a body plunging into deep water", 2.0, 0.3),
    ("item_pickup",  "a small item picked up, soft pop blip, game pickup", 0.5, 0.3),
    ("craft_done",   "crafting finished, two confident taps then a soft cloth swish", 1.5, 0.3),
    ("chest_open",   "an old wooden chest creaking open", 2.0, 0.3),
    ("smith_clang",  "a blacksmith hammer striking an anvil, bright metallic ring", 1.5, 0.3),
    ("player_death", "a somber death sting, a low tone descending and fading", 3.0, 0.3),
]


def looks_like_mpeg(data: bytes) -> bool:
    return data[:3] == b"ID3" or (len(data) > 2 and data[0] == 0xFF and (data[1] & 0xE0) == 0xE0)


def generate(name: str, prompt: str, dur: float, influence: float, key: str) -> bool:
    body = json.dumps({
        "text": prompt,
        "duration_seconds": dur,
        "prompt_influence": influence,
    }).encode()
    req = urllib.request.Request(
        API,
        data=body,
        headers={"xi-api-key": key, "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=180) as resp:
            data = resp.read()
    except urllib.error.HTTPError as e:
        detail = e.read()[:200].decode(errors="replace")
        print(f"  FAIL {name}: HTTP {e.code} {detail}")
        return False
    except (urllib.error.URLError, TimeoutError) as e:
        print(f"  FAIL {name}: network {e}")
        return False
    if not looks_like_mpeg(data):
        print(f"  FAIL {name}: response is not MPEG audio ({len(data)} bytes, head={data[:16]!r})")
        return False
    with open(os.path.join(OUT, name + ".mp3"), "wb") as f:
        f.write(data)
    print(f"  ok   {name} ({len(data)} bytes)")
    return True


def main() -> int:
    key = os.environ.get("ELEVENLABS_API_KEY")
    if not key:
        print("ELEVENLABS_API_KEY not set", file=sys.stderr)
        return 2
    os.makedirs(OUT, exist_ok=True)
    wanted = set(sys.argv[1:])
    todo = [
        (n, p, d, i)
        for (n, p, d, i) in MANIFEST
        if not wanted or n in wanted
    ]
    missing = [
        t for t in todo
        if not (os.path.getsize(os.path.join(OUT, t[0] + ".mp3")) > 512
                if os.path.exists(os.path.join(OUT, t[0] + ".mp3")) else False)
    ]
    print(f"{len(todo)} in manifest, {len(todo) - len(missing)} cached, {len(missing)} to generate")
    ok = 0
    for n, p, d, i in missing:
        if generate(n, p, d, i, key):
            ok += 1
        time.sleep(1.0)  # be gentle with the API
    print(f"done: {ok}/{len(missing)} generated")
    return 0 if ok == len(missing) else 1


if __name__ == "__main__":
    sys.exit(main())
