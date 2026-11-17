"""Require a fresh focused GPU presentation after this launch's native resume."""
import re

RESUME = re.compile(r"\bhost=(\d+) pid=(\d+) resumed\b")
PRESENT = re.compile(r"\bhost=(\d+) pid=(\d+) presented \d+x\d+ scale=([\d.]+) safe_top_px=([\d.]+) focused=(true|false)\b")
INVALIDATE = re.compile(r"\bhost=(\d+) pid=(\d+) (?:paused|surface-created|surface-destroyed|destroyed|returned|focus-lost)\b")
PHASE = re.compile(r"\bhost=\d+ pid=\d+ (resumed|paused|destroyed|returned)\b")


def paused_since(before, trace):
    phases = [m.group(1) for line in trace.splitlines()
              if line not in before and (m := PHASE.search(line))]
    return bool(phases) and phases[-1] == "paused"


def focused_presentation(before, trace, scale, safe_top):
    """Log lines include logcat timestamps, so identical old messages stay stale.

    The caller filters trace to the live gallery PID. A later resume supersedes
    earlier hosts/frames, including activity recreation in the same process.
    """
    lines = trace.splitlines()
    resumed = [(i, m.groups()) for i, line in enumerate(lines)
               if line not in before and (m := RESUME.search(line))]
    if not resumed:
        return False
    index, host = resumed[-1]
    latest = None
    for line in lines[index + 1:]:
        if line in before:
            continue
        invalidated = INVALIDATE.search(line)
        if invalidated and invalidated.groups() == host:
            latest = None
        match = PRESENT.search(line)
        if not match or match.groups()[:2] != host:
            continue
        latest = match.groups()[2:]
    if latest is None:
        return False
    density, inset, focused = latest
    return focused == "true" and abs(float(density) - scale) < 0.01 and abs(float(inset) - safe_top) < 1
