"""Generate the animated diagram: python3 docs/diagrams/sluice.py docs/diagrams/sluice.svg

One 16 s loop, CSS keyframes only, so it animates inside an <img> on GitHub.
The blueprints book carries a copy at books/sluice/src/images/sluice.svg.
"""
import sys

T = 16  # seconds per loop
E = 0.8  # fade width, percent
css = []


def kf(name, frames):
    body = " ".join(f"{p:g}%{{{v}}}" for p, v in frames)
    css.append(f"@keyframes {name}{{{body}}}")


def show(name, a, b, lo=0, hi=1):
    """Opacity hi inside [a, b], lo outside."""
    if a <= 0:
        fr = [(0, f"opacity:{hi}"), (b, f"opacity:{hi}"), (b + E, f"opacity:{lo}"),
              (100 - E, f"opacity:{lo}"), (100, f"opacity:{hi}")]
    else:
        fr = [(0, f"opacity:{lo}"), (a - E, f"opacity:{lo}"), (a, f"opacity:{hi}"),
              (b, f"opacity:{hi}"), (min(b + E, 100), f"opacity:{lo}")]
        if b + E < 100:
            fr.append((100, f"opacity:{lo}"))
    kf(name, fr)
    return name


def anim(cls, name):
    css.append(f".{cls}{{animation:{name} {T}s linear infinite}}")


# ---- timeline (percent of the loop) --------------------------------------
# 2-18 submit · 18-42 test · 42-56 attribute · 56-92 land · 96-100 reset
svg = []
add = svg.append

# sessions -----------------------------------------------------------------
sessions = [("A", "unit/auth", 80, 4, (6, 56), (58, 96), "landed", "ok"),
            ("B", "unit/ledger", 170, 7, (9, 45), (46, 96), "told: pkg-x:test failed", "bad"),
            ("C", "unit/export", 260, 10, (12, 80), (82, 96), "landed", "ok")]
for s, branch, y, sub, wait, end, endtxt, endcol in sessions:
    add(f'<rect class="box sb{s}" x="20" y="{y}" width="200" height="72" rx="6"/>')
    add(f'<text class="t b" x="34" y="{y+24}">agent {s}</text>')
    add(f'<text class="m sm" x="34" y="{y+42}">worktree {branch}</text>')
    anim(f"s{s}0", show(f"k-s{s}0", 0, sub))
    anim(f"s{s}1", show(f"k-s{s}1", *wait))
    anim(f"s{s}2", show(f"k-s{s}2", *end))
    add(f'<text class="m xs s{s}0" x="34" y="{y+60}">building…</text>')
    add(f'<text class="m xs s{s}1" x="34" y="{y+60}">submitted · waiting</text>')
    add(f'<text class="xs {endcol} s{s}2" x="34" y="{y+60}">{endtxt}</text>')
    a, b = end
    kf(f"k-sb{s}", [(0, "stroke:var(--rule)"), (a - E, "stroke:var(--rule)"),
                    (a, f"stroke:var(--{endcol})"), (b, f"stroke:var(--{endcol})"),
                    (b + E, "stroke:var(--rule)"), (100, "stroke:var(--rule)")])
    anim(f"sb{s}", f"k-sb{s}")

# entry chips: fly from each session into the queue -------------------------
chips = [("A", 360, 80, 4, 58), ("B", 400, 170, 7, 58), ("C", 440, 260, 10, 82)]
for s, cx, sy, t0, gone in chips:
    dx, dy = 170 - cx, (sy + 8) - 72
    kf(f"k-c{s}", [(0, f"transform:translate({dx}px,{dy}px);opacity:0"),
                   (t0 - 1, f"transform:translate({dx}px,{dy}px);opacity:0"),
                   (t0, f"transform:translate({dx}px,{dy}px);opacity:1"),
                   (t0 + 6, "transform:translate(0px,0px);opacity:1"),
                   (gone - E, "transform:translate(0px,0px);opacity:1"),
                   (gone, "transform:translate(0px,0px);opacity:0"),
                   (100, "transform:translate(0px,0px);opacity:0")])
    anim(f"c{s}", f"k-c{s}")
    add(f'<g class="c{s}"><rect class="chip chipr{s}" x="{cx}" y="72" width="30" height="22" rx="4"/>'
        f'<text class="chipt" x="{cx+15}" y="87" text-anchor="middle">{s}</text></g>')
kf("k-chiprB", [(0, "fill:var(--accent)"), (44, "fill:var(--accent)"), (45, "fill:var(--bad)"),
                (100, "fill:var(--bad)")])
anim("chiprB", "k-chiprB")

# candidate rows -------------------------------------------------------------
rows_y = [112, 192, 272]
kf("k-row", [(0, "opacity:0"), (18, "opacity:0"), (21, "opacity:1"), (96, "opacity:1"), (100, "opacity:0")])
kf("k-row2", [(0, "opacity:0"), (18, "opacity:0"), (21, "opacity:1"), (56, "opacity:1"),
              (60, "opacity:.3"), (96, "opacity:.3"), (100, "opacity:0")])
anim("row", "k-row")
anim("row2", "k-row2")


def row(i, cls, labels, statuses, barcls):
    y = rows_y[i]
    add(f'<g class="{cls}">')
    add(f'<rect class="cand" x="282" y="{y}" width="352" height="62" rx="6"/>')
    for txt, c in labels:
        add(f'<text class="t mono {c}" x="296" y="{y+24}">{txt}</text>')
    for txt, col, c in statuses:
        add(f'<text class="xs {col} {c}" x="620" y="{y+24}" text-anchor="end">{txt}</text>')
    add(f'<rect class="track" x="296" y="{y+38}" width="324" height="10" rx="5"/>')
    add(f'<rect class="bar {barcls}" x="296" y="{y+38}" width="324" height="10" rx="5"/>')
    if i == 1:
        add(f'<line class="strike" x1="290" y1="{y+20}" x2="410" y2="{y+20}"/>')
    add('</g>')


def status(cls, a, b):
    anim(cls, show(f"k-{cls}", a, b))
    return cls


row(0, "row", [("branch + A", "")],
    [("testing…", "m", status("r1a", 22, 36)), ("passed", "ok", status("r1b", 36, 56)),
     ("landed", "ok", status("r1c", 56, 96))], "bar1")
row(1, "row2", [("branch + A + B", "")],
    [("testing…", "m", status("r2a", 22, 32)), ("failed: pkg-x:test", "bad", status("r2b", 32, 56)),
     ("B removed: culprit", "bad", status("r2c", 56, 96))], "bar2")
row(2, "row", [("branch + A + B + C", status("l3a", 0, 60)), ("branch + A + C", status("l3b", 62, 96))],
    [("testing…", "m", status("r3a", 22, 42)), ("discarded: B is ahead", "m", status("r3b", 42, 62)),
     ("re-formed · 9 of 11 replay", "m", status("r3c", 62, 70)), ("passed", "ok", status("r3d", 70, 80)),
     ("landed", "ok", status("r3e", 80, 96))], "bar3")
kf("k-strike", [(0, "opacity:0"), (56, "opacity:0"), (58, "opacity:1"), (100, "opacity:1")])
anim("strike", "k-strike")

kf("k-bar1", [(0, "transform:scaleX(0);fill:var(--accent)"), (20, "transform:scaleX(0);fill:var(--accent)"),
              (36, "transform:scaleX(1);fill:var(--accent)"), (37, "transform:scaleX(1);fill:var(--ok)"),
              (96, "transform:scaleX(1);fill:var(--ok)"), (100, "transform:scaleX(0);fill:var(--accent)")])
kf("k-bar2", [(0, "transform:scaleX(0);fill:var(--accent)"), (20, "transform:scaleX(0);fill:var(--accent)"),
              (32, "transform:scaleX(.68);fill:var(--accent)"), (33, "transform:scaleX(.68);fill:var(--bad)"),
              (96, "transform:scaleX(.68);fill:var(--bad)"), (100, "transform:scaleX(0);fill:var(--accent)")])
kf("k-bar3", [(0, "transform:scaleX(0);fill:var(--accent);opacity:1"),
              (20, "transform:scaleX(0);fill:var(--accent);opacity:1"),
              (42, "transform:scaleX(.8);fill:var(--accent);opacity:1"),
              (44, "transform:scaleX(.8);fill:var(--accent);opacity:.25"),
              (61, "transform:scaleX(.8);fill:var(--accent);opacity:.25"),
              (62, "transform:scaleX(0);fill:var(--accent);opacity:1"),
              (64, "transform:scaleX(0);fill:var(--accent);opacity:1"),
              (70, "transform:scaleX(1);fill:var(--accent);opacity:1"),
              (71, "transform:scaleX(1);fill:var(--ok);opacity:1"),
              (96, "transform:scaleX(1);fill:var(--ok);opacity:1"),
              (100, "transform:scaleX(0);fill:var(--accent);opacity:1")])
for b in ("bar1", "bar2", "bar3"):
    anim(b, f"k-{b}")

# the failure goes to B's agent and no other ---------------------------------
add('<path class="notify" d="M282,222 C250,222 250,208 222,208"/>')
kf("k-notify", [(0, "stroke-dashoffset:80;opacity:0"), (44, "stroke-dashoffset:80;opacity:0"),
                (44.5, "stroke-dashoffset:80;opacity:1"), (47, "stroke-dashoffset:0;opacity:1"),
                (96, "stroke-dashoffset:0;opacity:1"), (97, "stroke-dashoffset:0;opacity:0"),
                (100, "stroke-dashoffset:80;opacity:0")])
anim("notify", "k-notify")

# the branch ---------------------------------------------------------------------
LX = 760
add(f'<line class="edge" x1="{LX}" y1="340" x2="{LX}" y2="322"/>')
add(f'<line class="edge eA" x1="{LX}" y1="312" x2="{LX}" y2="260"/>')
add(f'<line class="edge eC" x1="{LX}" y1="242" x2="{LX}" y2="190"/>')
add(f'<circle class="commit" cx="{LX}" cy="317" r="6"/>')
add(f'<text class="m xs" x="{LX+14}" y="321">before</text>')
for s, y, a in (("A", 251, 56), ("C", 181, 80)):
    anim(f"m{s}", show(f"k-m{s}", a, 96))
    add(f'<g class="m{s}"><circle class="merge" cx="{LX}" cy="{y}" r="8"/>'
        f'<circle class="mergein" cx="{LX}" cy="{y}" r="3.5"/>'
        f'<text class="t sm" x="{LX+16}" y="{y-2}">merge {s}</text>'
        f'<text class="m xs" x="{LX+16}" y="{y+13}">verdict in trailers</text></g>')
anim("eA", show("k-eA", 56, 96))
anim("eC", show("k-eC", 80, 96))
kf("k-head", [(0, "transform:translate(0px,0px)"), (56, "transform:translate(0px,0px)"),
              (58, "transform:translate(0px,-66px)"), (80, "transform:translate(0px,-66px)"),
              (82, "transform:translate(0px,-136px)"), (96, "transform:translate(0px,-136px)"),
              (99, "transform:translate(0px,0px)"), (100, "transform:translate(0px,0px)")])
anim("head", "k-head")
add(f'<g class="head"><rect class="pill" x="664" y="306" width="76" height="22" rx="11"/>'
    f'<text class="pillt" x="702" y="321" text-anchor="middle">bolt/pay</text></g>')
add(f'<text class="m xs" x="664" y="356">moved only by sluice, from the</text>')
add(f'<text class="m xs" x="664" y="370">revision the candidate was formed on</text>')

# captions -------------------------------------------------------------------
caps = [("1", "Agents submit their work.", "None of them merges.", 2, 18),
        ("2", "Each candidate is tested", "exactly as it would land.", 18, 42),
        ("3", "A failure has one culprit,", "and only its agent is told.", 42, 56),
        ("4", "The branch moves only", "to what passed.", 56, 92)]
for i, (n, l1, l2, a, b) in enumerate(caps):
    x = 20 + i * 218
    anim(f"cap{n}", show(f"k-cap{n}", a, b, lo=.35))
    add(f'<g class="cap cap{n}"><circle class="badge" cx="{x+12}" cy="412" r="11"/>'
        f'<text class="badget" x="{x+12}" y="416" text-anchor="middle">{n}</text>'
        f'<text class="t sm" x="{x+30}" y="408">{l1}</text>'
        f'<text class="t sm" x="{x+30}" y="425">{l2}</text></g>')

STYLE = """
:root{--bg:#f6f7f8;--panel:#ffffff;--fg:#1d2329;--muted:#5b6670;--rule:#d6dbe0;
--accent:#1b6d8a;--soft:#e3eef3;--ok:#2e7d4f;--bad:#c0392b;--ink:#ffffff}
@media (prefers-color-scheme:dark){:root{--bg:#141a1f;--panel:#1b2329;--fg:#e6ebef;
--muted:#9aa7b2;--rule:#2c3740;--accent:#6fc2de;--soft:#1f2f38;--ok:#6cc58f;--bad:#f08a7e;--ink:#0f1a20}}
text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Helvetica,Arial,sans-serif;fill:var(--fg)}
.mono,.chipt,.pillt{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.bg{fill:var(--bg)} .t{fill:var(--fg)} .m{fill:var(--muted)} .ok{fill:var(--ok)} .bad{fill:var(--bad)}
.b{font-weight:600} .h{font-size:17px;font-weight:700} .hd{font-size:12px;letter-spacing:.06em;fill:var(--muted)}
.sm{font-size:13px} .xs{font-size:11.5px} .mono{font-size:13px}
.box{fill:var(--panel);stroke:var(--rule);stroke-width:1.5}
.cand{fill:var(--panel);stroke:var(--rule);stroke-width:1}
.queue{fill:var(--soft);stroke:none}
.chip{fill:var(--accent)} .chipt{fill:var(--ink);font-size:13px;font-weight:700}
.track{fill:var(--soft)} .bar{transform-box:fill-box;transform-origin:left center}
.strike{stroke:var(--bad);stroke-width:1.5}
.notify{fill:none;stroke:var(--bad);stroke-width:2;stroke-dasharray:80}
.edge{stroke:var(--muted);stroke-width:2} .commit{fill:var(--muted)}
.merge{fill:var(--ok)} .mergein{fill:var(--panel)}
.pill{fill:var(--accent)} .pillt{fill:var(--ink);font-size:12px;font-weight:600}
.badge{fill:var(--accent)} .badget{fill:var(--ink);font-size:12px;font-weight:700}
.arrowhead{fill:var(--bad)}
@media (prefers-reduced-motion:reduce){*{animation-play-state:paused!important;animation-delay:-14.4s!important}
.cap{animation:none!important;opacity:1}}
"""

head = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 446" width="900" height="446" role="img" aria-labelledby="ttl dsc">
<title id="ttl">sluice: a merge queue for agents in worktrees</title>
<desc id="dsc">Three agents, each in its own worktree, submit their work and never merge. sluice forms candidates branch plus A, branch plus A plus B, and branch plus A plus B plus C, and tests them at once. Branch plus A plus B fails, so B is the one culprit and only agent B is told. A lands as a merge commit; C is re-formed on the branch without B, mostly replays from the cache, passes and lands. The branch moves only to what passed.</desc>
<style>{STYLE}{"".join(css)}</style>
<rect class="bg" width="900" height="446" rx="10"/>
<text class="h" x="20" y="32">sluice</text>
<text class="m sm" x="80" y="32">a merge queue for agents in worktrees</text>
<text class="hd" x="20" y="62">AGENTS · EACH IN A WORKTREE</text>
<rect class="queue" x="268" y="44" width="380" height="306" rx="8"/>
<text class="hd" x="282" y="62">SLUICE · QUEUE OF bolt/pay</text>
<text class="m xs" x="296" y="88">entries</text>
<text class="hd" x="664" y="62">THE BRANCH</text>
'''
out = head + "\n".join(svg) + "\n</svg>\n"
open(sys.argv[1], "w").write(out)
