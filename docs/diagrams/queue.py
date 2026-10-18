"""Generate the queue's timing diagrams: python3 docs/diagrams/queue.py docs/diagrams

Writes modes.svg, failure.svg and at-once.svg, the diagrams of the guide
"Parallel and batch" (site/src/content/docs/guides/parallel-and-batch.md).
Each is one loop of CSS keyframes only, as sluice.py's is, so it animates
inside an <img> on GitHub; the palette is sluice.py's.
"""

import os
import sys

T = 18  # seconds per loop
SWEEP = 82  # percent of the loop the playhead takes to cross the axis
HOLD = 97  # percent at which everything resets
E = 0.6  # fade width, percent

STYLE = """
:root{--bg:#ebe9e2;--panel:#fbfaf7;--fg:#0e1116;--muted:#5d6370;--rule:#d5d2c8;
--accent:#2b313c;--soft:#e0ddd4;--ok:#1f8a4c;--bad:#c0392b;--ink:#f2f1ec}
@media (prefers-color-scheme:dark){:root{--bg:#14181f;--panel:#1b2029;--fg:#f2f1ec;
--muted:#9a9fab;--rule:#2c323d;--accent:#c9cbd2;--soft:#232933;--ok:#45c27a;--bad:#f08a7e;--ink:#0e1116}}
text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Helvetica,Arial,sans-serif;fill:var(--fg)}
.mono,.bart{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.bg{fill:var(--bg)} .t{fill:var(--fg)} .m{fill:var(--muted)} .ok{fill:var(--ok)} .bad{fill:var(--bad)}
.b{font-weight:600} .h{font-size:17px;font-weight:700} .hd{font-size:12px;letter-spacing:.06em;fill:var(--muted)}
.sm{font-size:13px} .xs{font-size:11.5px} .mono{font-size:12.5px}
.panel{fill:var(--soft);stroke:none}
.track{fill:var(--panel);stroke:var(--rule);stroke-width:1}
.bar{transform-box:fill-box;transform-origin:left center}
.bart{fill:var(--ink);font-size:11.5px;font-weight:600}
.wait{stroke:var(--muted);stroke-width:2;stroke-dasharray:4 4;transform-box:fill-box;transform-origin:left center}
.axis{stroke:var(--rule);stroke-width:1} .tick{fill:var(--muted);font-size:11px}
.play{stroke:var(--accent);stroke-width:1.5;stroke-dasharray:3 3}
.merge{fill:var(--ok)} .mergein{fill:var(--panel)}
.badge{fill:var(--accent)} .badget{fill:var(--ink);font-size:12px;font-weight:700}
.step{fill:none;stroke:var(--accent);stroke-width:3;stroke-linejoin:round}
.dot{fill:var(--accent)} .grid{stroke:var(--rule);stroke-width:1}
.slot{fill:var(--panel);stroke:var(--rule);stroke-width:1.5} .used{fill:var(--accent)}
@media (prefers-reduced-motion:reduce){*{animation-play-state:paused!important;animation-delay:-16.5s!important}
.cap{animation:none!important;opacity:1}}
"""


class Svg:
    def __init__(self, w, h, title, desc):
        self.w, self.h, self.title, self.desc = w, h, title, desc
        self.css, self.body, self.n = [], [], 0

    def add(self, s):
        self.body.append(s)

    def kf(self, frames):
        self.n += 1
        name = f"k{self.n}"
        body = " ".join(f"{p:g}%{{{v}}}" for p, v in frames)
        self.css.append(f"@keyframes {name}{{{body}}}")
        self.css.append(f".{name}{{animation:{name} {T}s linear infinite}}")
        return name

    def show(self, a, b=HOLD, lo=0, hi=1):
        """A class whose opacity is hi inside [a, b] percent, lo outside."""
        return self.kf(
            [
                (0, f"opacity:{lo}"),
                (max(a - E, 0), f"opacity:{lo}"),
                (a, f"opacity:{hi}"),
                (b, f"opacity:{hi}"),
                (min(b + E, 100), f"opacity:{lo}"),
                (100, f"opacity:{lo}"),
            ]
        )

    def write(self, path):
        out = (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" '
            f'width="{self.w}" height="{self.h}" role="img" aria-labelledby="ttl dsc">\n'
            f'<title id="ttl">{self.title}</title>\n<desc id="dsc">{self.desc}</desc>\n'
            f"<style>{STYLE}{''.join(self.css)}</style>\n"
            f'<rect class="bg" width="{self.w}" height="{self.h}" rx="10"/>\n'
            + "\n".join(self.body)
            + "\n</svg>\n"
        )
        with open(path, "w") as f:
            f.write(out)


class Timeline:
    """Runs drawn against minutes, filled in as a playhead crosses them."""

    X0, X1 = 250, 870

    def __init__(self, svg, minutes):
        self.s, self.minutes = svg, minutes

    def x(self, m):
        return self.X0 + (self.X1 - self.X0) * m / self.minutes

    def p(self, m):
        """The loop's percent at which the playhead reaches minute m."""
        return 2 + (SWEEP - 2) * m / self.minutes

    def label(self, y, text, cls="t mono"):
        self.s.add(f'<text class="{cls}" x="34" y="{y + 15}">{text}</text>')

    def run(self, y, a, b, end, said, thrown=None):
        """A run from minute a to b: it ends `end` (ok, bad) and says `said`.
        A run thrown away at minute `thrown` fades instead."""
        s, x0, x1 = self.s, self.x(a), self.x(b)
        w = x1 - x0
        s.add(
            f'<rect class="track" x="{x0:.1f}" y="{y}" width="{w:.1f}" height="20" rx="4"/>'
        )
        pa, pb = self.p(a), self.p(b)
        grow = s.kf(
            [
                (0, "transform:scaleX(0);fill:var(--accent)"),
                (pa, "transform:scaleX(0);fill:var(--accent)"),
                (pb, "transform:scaleX(1);fill:var(--accent)"),
                (pb + 0.4, f"transform:scaleX(1);fill:var(--{end})"),
                (HOLD, f"transform:scaleX(1);fill:var(--{end})"),
                (100, "transform:scaleX(0);fill:var(--accent)"),
            ]
        )
        s.add(
            f'<rect class="bar {grow}" x="{x0:.1f}" y="{y}" width="{w:.1f}" height="20" rx="4"/>'
        )
        s.add(
            f'<text class="bart {s.show(pb + 0.4)}" x="{x0 + 8:.1f}" y="{y + 14}">{said}</text>'
        )

    def thrown(self, y, a, b, said):
        """A run started at a and thrown away at b, still unfinished."""
        s, x0 = self.s, self.x(a)
        full = self.x(a + 20) - x0
        s.add(
            f'<rect class="track" x="{x0:.1f}" y="{y}" width="{full:.1f}" height="20" rx="4"/>'
        )
        pa, pb = self.p(a), self.p(b)
        frac = (b - a) / 20
        grow = s.kf(
            [
                (0, "transform:scaleX(0);opacity:1"),
                (pa, "transform:scaleX(0);opacity:1"),
                (pb, f"transform:scaleX({frac:.3f});opacity:1"),
                (pb + 1, f"transform:scaleX({frac:.3f});opacity:.25"),
                (HOLD, f"transform:scaleX({frac:.3f});opacity:.25"),
                (100, "transform:scaleX(0);opacity:1"),
            ]
        )
        s.add(
            f'<rect class="bar {grow}" style="fill:var(--muted)" x="{x0:.1f}" y="{y}" '
            f'width="{full:.1f}" height="20" rx="4"/>'
        )
        s.add(
            f'<text class="m xs {s.show(pb + 0.4)}" x="{x0 + 8:.1f}" y="{y + 14}">{said}</text>'
        )

    def wait(self, y, a, b, said):
        """Submissions queued from minute a to b, waiting for a run."""
        s, x0, x1 = self.s, self.x(a), self.x(b)
        grow = s.kf(
            [
                (0, "transform:scaleX(0)"),
                (self.p(a), "transform:scaleX(0)"),
                (self.p(b), "transform:scaleX(1)"),
                (HOLD, "transform:scaleX(1)"),
                (100, "transform:scaleX(0)"),
            ]
        )
        s.add(
            f'<line class="wait {grow}" x1="{x0:.1f}" y1="{y + 10}" x2="{x1:.1f}" y2="{y + 10}"/>'
        )
        s.add(
            f'<text class="m xs {s.show(self.p(a) + 1)}" x="{x0 + 4:.1f}" y="{y + 4}">{said}</text>'
        )

    def land(self, y, m):
        s, cx = self.s, self.x(m)
        s.add(
            f'<g class="{s.show(self.p(m) + 0.4)}"><circle class="merge" cx="{cx:.1f}" cy="{y + 10}" r="7"/>'
            f'<circle class="mergein" cx="{cx:.1f}" cy="{y + 10}" r="3"/></g>'
        )

    def note(self, y, m, text, cls="m xs"):
        """Text that appears when the playhead reaches minute m."""
        self.s.add(
            f'<text class="{cls} {self.s.show(self.p(m) + 0.4)}" x="34" y="{y}">{text}</text>'
        )

    def axis(self, y, top, step):
        s = self.s
        s.add(f'<line class="axis" x1="{self.X0}" y1="{y}" x2="{self.X1}" y2="{y}"/>')
        for m in range(0, self.minutes + 1, step):
            x = self.x(m)
            s.add(
                f'<line class="axis" x1="{x:.1f}" y1="{y}" x2="{x:.1f}" y2="{y + 5}"/>'
            )
            s.add(
                f'<text class="tick" x="{x:.1f}" y="{y + 18}" text-anchor="middle">{m} min</text>'
            )
        move = s.kf(
            [
                (0, "transform:translateX(0px)"),
                (2, "transform:translateX(0px)"),
                (SWEEP, f"transform:translateX({self.X1 - self.X0}px)"),
                (HOLD, f"transform:translateX({self.X1 - self.X0}px)"),
                (100, "transform:translateX(0px)"),
            ]
        )
        s.add(
            f'<line class="play {move}" x1="{self.X0}" y1="{top}" x2="{self.X0}" y2="{y}"/>'
        )


def captions(s, y, caps):
    for i, (n, l1, l2, a, b) in enumerate(caps):
        x = 20 + i * 292
        s.add(
            f'<g class="cap {s.show(a, b, lo=0.35)}"><circle class="badge" cx="{x + 12}" cy="{y + 4}" r="11"/>'
            f'<text class="badget" x="{x + 12}" y="{y + 8}" text-anchor="middle">{n}</text>'
            f'<text class="t sm" x="{x + 30}" y="{y}">{l1}</text>'
            f'<text class="t sm" x="{x + 30}" y="{y + 17}">{l2}</text></g>'
        )


def modes(path):
    s = Svg(
        900,
        470,
        "Parallel and batch: four submissions on a 20-minute gate",
        "Four submissions, A to D, arrive a minute apart on a branch whose tests take "
        "20 minutes. With batch false and parallel 4, sluice tests main plus A, main plus A "
        "plus B, and so on, four runs at once, and all four land by minute 23. With batch "
        "true, main plus A starts alone; B, C and D wait while it is under test, then are "
        "tested together as one run that lands all three at minute 40: two runs, one at a time.",
    )
    t = Timeline(s, 45)
    s.add('<text class="h" x="20" y="32">Four submissions, a 20-minute gate</text>')
    s.add(
        '<text class="m sm" x="20" y="52">A, B, C and D are submitted a minute apart.</text>'
    )

    s.add('<rect class="panel" x="16" y="66" width="868" height="148" rx="8"/>')
    s.add('<text class="hd" x="34" y="86">BATCH = FALSE · PARALLEL = 4</text>')
    for i, (lab, a) in enumerate(
        [
            ("main + A", 0),
            ("main + A + B", 1),
            ("main + A + B + C", 2),
            ("main + A + B + C + D", 3),
        ]
    ):
        y = 96 + i * 26
        t.label(y, lab)
        t.run(y, a, a + 20, "ok", f"passed · {'ABCD'[i]} lands")
        t.land(y, a + 20)
    s.add(
        f'<text class="b sm ok {s.show(t.p(23) + 0.4)}" x="560" y="86">4 runs at once · '
        f"all landed by 23 min</text>"
    )

    s.add('<rect class="panel" x="16" y="226" width="868" height="122" rx="8"/>')
    s.add('<text class="hd" x="34" y="246">BATCH = TRUE · PARALLEL = 4</text>')
    t.label(258, "main + A")
    t.run(258, 0, 20, "ok", "passed · A lands")
    t.land(258, 20)
    t.label(296, "main + B + C + D")
    t.wait(296, 1, 20, "B, C, D queue while A is tested")
    t.run(296, 20, 40, "ok", "passed · B, C, D land together")
    t.land(296, 40)
    s.add(
        f'<text class="b sm ok {s.show(t.p(40) + 0.4)}" x="560" y="246">2 runs, one at a time · '
        f"all landed by 40 min</text>"
    )

    t.axis(360, 66, 5)
    captions(
        s,
        418,
        [
            (
                "1",
                "Parallel bets the ones ahead pass:",
                "each starts on arrival.",
                2,
                30,
            ),
            (
                "2",
                "Batch tests one run at a time;",
                "what queues meanwhile is the next batch.",
                30,
                62,
            ),
            ("3", "Parallel lands sooner;", "batch uses fewer runs.", 62, HOLD),
        ],
    )
    s.write(path)


def failure(path):
    s = Svg(
        900,
        500,
        "Parallel and batch: a failure in each",
        "A, B and C are queued together on a branch whose tests take 20 minutes, and B "
        "breaks the build. With batch false, main plus A, plus A plus B and plus A plus B "
        "plus C are tested at once; at minute 20 A lands, B is blamed, the run with B in it "
        "is thrown away, and C is tested again on the new main, landing at minute 40. With "
        "batch true, only main plus A plus B plus C is tested; it fails at minute 20, so "
        "the two shorter combinations it skipped are tested; at minute 40 A lands and B is "
        "blamed, and C is tested again, landing at minute 60.",
    )
    t = Timeline(s, 62)
    s.add('<text class="h" x="20" y="32">A failure in each mode</text>')
    s.add(
        '<text class="m sm" x="20" y="52">A, B and C are queued together, the gate takes '
        "20 minutes, and B breaks the build.</text>"
    )

    s.add('<rect class="panel" x="16" y="66" width="868" height="150" rx="8"/>')
    s.add('<text class="hd" x="34" y="86">BATCH = FALSE · PARALLEL = 4</text>')
    t.label(96, "main + A")
    t.run(96, 0, 20, "ok", "passed · A lands")
    t.land(96, 20)
    t.label(122, "main + A + B")
    t.run(122, 0, 20, "bad", "failed · B is blamed")
    t.label(148, "main + A + B + C")
    t.thrown(148, 0, 20, "thrown away: B is in it")
    t.label(174, "main + C")
    t.run(174, 20, 40, "ok", "passed · C lands")
    t.land(174, 40)
    s.add(
        f'<text class="b sm {s.show(t.p(40) + 0.4)}" x="560" y="86">B told at 20 min · '
        f"C landed at 40 min</text>"
    )

    s.add('<rect class="panel" x="16" y="228" width="868" height="150" rx="8"/>')
    s.add('<text class="hd" x="34" y="248">BATCH = TRUE · PARALLEL = 4</text>')
    t.label(258, "main + A + B + C")
    t.run(258, 0, 20, "bad", "failed · which one?")
    t.label(284, "main + A")
    t.run(284, 20, 40, "ok", "passed · A lands")
    t.land(284, 40)
    t.label(310, "main + A + B")
    t.run(310, 20, 40, "bad", "failed · B is blamed")
    t.label(336, "main + C")
    t.run(336, 40, 60, "ok", "passed · C lands")
    t.land(336, 60)
    s.add(
        f'<text class="b sm {s.show(t.p(60) + 0.4)}" x="560" y="248">B told at 40 min · '
        f"C landed at 60 min</text>"
    )

    t.axis(392, 66, 10)
    captions(
        s,
        448,
        [
            (
                "1",
                "Parallel: the failure names B",
                "at once; the run behind it is wasted.",
                2,
                34,
            ),
            (
                "2",
                "Batch: the longest fails, so the",
                "shorter ones run to find who broke it.",
                34,
                62,
            ),
            (
                "3",
                "Either way, only B's agent is told,",
                "and C lands without B.",
                62,
                HOLD,
            ),
        ],
    )
    s.write(path)


def at_once(path):
    s = Svg(
        900,
        440,
        "Tested at once, and the host's bound",
        "Left: how many of a branch's combinations are tested at once. It "
        "starts at parallel, 4; each failure halves it, to 2 then 1; each pass adds one back, "
        "to 2, 3 and 4. sluice status shows it as testing 1 at once, up to 4. Right: the "
        "host's bound, runs max, caps every branch together: here 4 runs, shared by main, "
        "two bolts and an agent's own sluice exec.",
    )
    s.add('<text class="h" x="20" y="32">How many are tested at once</text>')

    # left: how many a branch tests at once, over its verdicts -----------------
    s.add('<rect class="panel" x="16" y="48" width="520" height="300" rx="8"/>')
    s.add('<text class="hd" x="34" y="70">ONE BRANCH · PARALLEL = 4</text>')
    gx0, gx1, gy0, gy1 = 80, 510, 290, 110  # 0 at once at gy0, 4 at gy1

    def gy(w):
        return gy0 - (gy0 - gy1) * w / 4

    for w in range(1, 5):
        s.add(
            f'<line class="grid" x1="{gx0}" y1="{gy(w):.0f}" x2="{gx1}" y2="{gy(w):.0f}"/>'
        )
        s.add(
            f'<text class="tick" x="{gx0 - 10}" y="{gy(w) + 4:.0f}" text-anchor="end">{w}</text>'
        )
    s.add(f'<text class="m xs" x="34" y="{gy(4) - 14:.0f}">tested at once</text>')
    steps = [
        (4, None),
        (2, ("failed", "bad")),
        (1, ("failed", "bad")),
        (2, ("passed", "ok")),
        (3, ("passed", "ok")),
        (4, ("passed", "ok")),
        (4, ("passed", "ok")),
    ]
    dx = (gx1 - gx0) / len(steps)
    pts = []
    for i, (w, _) in enumerate(steps):
        xa, xb = gx0 + i * dx, gx0 + (i + 1) * dx
        pts += [(xa, gy(w)), (xb, gy(w))]
    d = "M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in pts)
    length = (
        sum(
            abs(pts[i + 1][0] - pts[i][0]) + abs(pts[i + 1][1] - pts[i][1])
            for i in range(len(pts) - 1)
        )
        + 10
    )
    draw = s.kf(
        [
            (0, f"stroke-dashoffset:{length:.0f}"),
            (4, f"stroke-dashoffset:{length:.0f}"),
            (SWEEP, "stroke-dashoffset:0"),
            (HOLD, "stroke-dashoffset:0"),
            (100, f"stroke-dashoffset:{length:.0f}"),
        ]
    )
    s.add(f'<path class="step {draw}" style="stroke-dasharray:{length:.0f}" d="{d}"/>')
    for i, (w, ev) in enumerate(steps):
        if ev is None:
            continue
        word, col = ev
        x = gx0 + i * dx
        at = 4 + (SWEEP - 4) * (i / len(steps))
        s.add(
            f'<text class="xs {col} {s.show(at)}" x="{x:.1f}" y="{gy0 + 22}" '
            f'text-anchor="middle">{word}</text>'
        )
        s.add(
            f'<text class="m xs {s.show(at)}" x="{x:.1f}" y="{gy0 + 37}" '
            f'text-anchor="middle">{"½" if word == "failed" else "+1" if w > steps[i - 1][0] else "at most 4"}</text>'
        )
    one = 4 + (SWEEP - 4) * (2 / len(steps))
    two = 4 + (SWEEP - 4) * (3 / len(steps))
    s.add(
        f'<g class="{s.show(one, two)}"><text class="t mono" x="300" y="{gy(1) - 34:.0f}">'
        f"testing 1 at once (up to 4)</text>"
        f'<text class="m xs" x="300" y="{gy(1) - 18:.0f}">what sluice status says now</text></g>'
    )

    # right: the host's bound ----------------------------------------------------
    s.add('<rect class="panel" x="548" y="48" width="336" height="300" rx="8"/>')
    s.add('<text class="hd" x="566" y="70">THE HOST · [runs] max = 4</text>')
    s.add(
        '<text class="m xs" x="566" y="90">every branch, and agents\' sluice exec, share it</text>'
    )
    users = [("main", 0, 2, 10), ("bolt/a", 1, 1, 30), ("sluice exec", 2, 1, 50)]
    for i in range(4):
        y = 108 + i * 50
        s.add(f'<rect class="slot" x="566" y="{y}" width="300" height="38" rx="6"/>')
        s.add(f'<text class="m xs" x="580" y="{y + 23}">run {i + 1}: free</text>')
    slot = 0
    for name, _, n, at in users:
        for _ in range(n):
            y = 108 + slot * 50
            s.add(
                f'<g class="{s.show(at)}"><rect class="used" x="566" y="{y}" width="300" height="38" rx="6"/>'
                f'<text class="bart" x="580" y="{y + 23}">run {slot + 1}: {name}</text></g>'
            )
            slot += 1
    s.add(
        f'<text class="xs bad {s.show(66)}" x="566" y="334">bolt/b waits for a free run, whatever its parallel</text>'
    )

    captions(
        s,
        384,
        [
            ("1", "A failure halves the number;", "a pass adds one back.", 2, 44),
            ("2", "It never falls below one,", "nor rises above parallel.", 44, 62),
            ("3", "The host's bound caps every", "branch together.", 62, HOLD),
        ],
    )
    s.write(path)


out = sys.argv[1]
modes(os.path.join(out, "modes.svg"))
failure(os.path.join(out, "failure.svg"))
at_once(os.path.join(out, "at-once.svg"))
