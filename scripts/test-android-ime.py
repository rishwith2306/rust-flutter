#!/usr/bin/env python3
"""Regression cases from the API 35 IME-readiness investigation."""
import unittest
from android_ime import ime_state


class ImeReadiness(unittest.TestCase):
    def test_api35_visible_keyboard_without_legacy_visibility_field(self):
        request = 'mCurId=com.google.android.inputmethod.latin/LatinIME\n  mInputShown=true\n'
        source = 'mSource=InsetsSource id=3 type=ime frame=[0,1517][1080,2400] visibleFrame=[0,1517][1080,2400] visible=true flags= sideHint=BOTTOM'
        self.assertEqual(ime_state(request, source), {"requested": True, "visible": True})

    def test_hidden_provider_takes_precedence_over_stale_window_snapshots(self):
        windows = 'InsetsSource id=3 type=ime visible=true\nmSource=InsetsSource id=3 type=ime frame=[0,0][0,0] visible=false flags='
        self.assertEqual(ime_state('mInputShown=false mImeWindowVis=3', windows), {"requested": False, "visible": False})

    def test_request_alone_does_not_establish_window_visibility(self):
        self.assertEqual(ime_state('mInputShown=true', 'mSource=InsetsSource id=3 type=ime visible=false'), {"requested": True, "visible": False})

    def test_missing_or_ambiguous_provider_is_unknown(self):
        self.assertIsNone(ime_state('mInputShown=true', 'InsetsSource id=3 type=ime visible=true'))
        self.assertIsNone(ime_state('mInputShown=false', 'mSource=InsetsSource id=3 type=ime visible=false\nmSource=InsetsSource id=4 type=ime visible=true'))


if __name__ == '__main__':
    unittest.main()
