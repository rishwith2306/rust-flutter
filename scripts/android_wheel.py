"""Validate separate-process OS wheel injection; pixels are checked elsewhere."""
import json
import math
import re


def wheel_result(text, returncode, before_pid, after_pid, steps):
    summaries = re.findall(r"^INSTRUMENTATION_RESULT: summary=(.*)$", text, re.MULTILINE)
    codes = re.findall(r"^INSTRUMENTATION_CODE:\s*(-?\d+)\s*$", text, re.MULTILINE)
    if returncode or len(summaries) != 1 or codes != ["-1"]:
        raise ValueError("wheel instrumentation did not finish successfully")
    value = json.loads(summaries[0])
    if not isinstance(value, dict) or value.get("passed") is not True:
        raise ValueError("wheel injection did not pass")
    if type(steps) is not int or not 1 <= abs(steps) <= 12:
        raise ValueError("invalid expected wheel steps")
    if type(value.get("injected")) is not int or value["injected"] != abs(steps):
        raise ValueError("wheel packet count mismatch")
    if value.get("source") != "SOURCE_MOUSE" or value.get("action") != "ACTION_SCROLL" or type(value.get("vertical_axis")) is not int or value["vertical_axis"] != (-1 if steps > 0 else 1):
        raise ValueError("unexpected OS scroll event description")
    for key in ("x", "y"):
        if type(value.get(key)) not in (int, float) or not math.isfinite(value[key]) or value[key] < 0:
            raise ValueError("invalid wheel coordinates")
    if not re.fullmatch(r"[1-9]\d*", before_pid) or before_pid != after_pid:
        raise ValueError("wheel runner changed the gallery process")
    runner = value.get("instrumentation_pid")
    if type(runner) is not int or runner <= 0 or str(runner) == after_pid:
        raise ValueError("wheel runner must run outside the gallery process")
    return {**value, "gallery_pid_before": before_pid, "gallery_pid_after": after_pid}
