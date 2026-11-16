"""Read IME readiness from its request plus the current Window Manager provider.
Do not use mImeWindowVis: it is absent from the API 35 input-method dump.
"""
import re


def ime_state(input_method, windows):
    requested = re.search(r"\bmInputShown=(true|false)\b", input_method)
    # Ignore per-window cached InsetsSource snapshots. The provider's mSource
    # is the current display state and is present on the tested Android releases.
    visible = re.findall(r"\bmSource=InsetsSource\b[^\n]*\btype=ime\b[^\n]*\bvisible=(true|false)\b", windows)
    if not requested or len(visible) != 1:
        return None
    return {"requested": requested.group(1) == "true", "visible": visible[0] == "true"}
