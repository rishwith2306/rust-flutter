"""Exercise the gallery's typed form using rendered controls and injected input."""
import time
from component_probe import bounds, center, patch


def probe_form(capture, tap, type_text, scale=1.0):
    def settled(name):
        time.sleep(0.4)
        return capture("form-" + name + ".png")

    def input_patch(path, rect):
        # TextEdit aligns text at the start, unlike the centered counter probes.
        left = (rect[0], rect[1], min(rect[2], 120 * scale), rect[3])
        return patch(path, left, scale, width=120)

    initial = settled("initial")
    editor = bounds(initial, "#263b4e")
    submit = bounds(initial, "#426b58")
    reset = bounds(initial, "#93643b")
    assert editor and submit and reset, "form controls are missing"
    empty = input_patch(initial, editor)
    tap(*center(submit))
    invalid = settled("invalid")
    assert bounds(invalid, "#b34d45") and not bounds(invalid, "#3f875e"), "empty form was not rejected"
    tap(*center(bounds(invalid, "#263b4e")))
    type_text("Ada")
    typed = settled("typed")
    assert input_patch(typed, bounds(typed, "#263b4e")) != empty, "typed input did not render"
    tap(*center(bounds(typed, "#426b58")))
    missing_terms = settled("missing-terms")
    assert bounds(missing_terms, "#b34d45") and not bounds(missing_terms, "#3f875e"), "unaccepted terms were not rejected"
    terms = bounds(missing_terms, "#624d77")
    assert terms, "missing terms field"
    tap(terms[0] + 10 * scale, terms[1] + 10 * scale)
    accepted = settled("accepted")
    tap(*center(bounds(accepted, "#426b58")))
    saved = settled("saved")
    assert bounds(saved, "#3f875e") and not bounds(saved, "#b34d45"), "valid form did not save"
    tap(*center(bounds(saved, "#93643b")))
    cleared = settled("reset")
    assert not bounds(cleared, "#3f875e") and not bounds(cleared, "#b34d45"), "reset kept submission status"
    assert input_patch(cleared, bounds(cleared, "#263b4e")) == empty, "reset did not clear the rendered input"
    tap(*center(bounds(cleared, "#426b58")))
    rejected_again = settled("rejected-again")
    assert bounds(rejected_again, "#b34d45") and not bounds(rejected_again, "#3f875e"), "reset form was not invalid again"
    return {"passed": True, "empty_rejected": True, "text_rendered": True, "terms_required": True, "saved": True, "reset_rendered": True, "reset_invalid_again": True}
