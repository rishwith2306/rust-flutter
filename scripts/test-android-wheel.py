#!/usr/bin/env python3
"""GPU-free regression checks for OS input evidence acceptance."""
import json
import unittest
from android_wheel import wheel_result


class WheelResultTest(unittest.TestCase):
    def record(self, **changes):
        return {"passed": True, "injected": 4, "x": 500.5, "y": 1200,
                "vertical_axis": -1, "source": "SOURCE_MOUSE", "action": "ACTION_SCROLL",
                "instrumentation_pid": 200, **changes}

    def output(self, value=None, code="-1"):
        return "INSTRUMENTATION_RESULT: summary=" + json.dumps(self.record() if value is None else value) + "\nINSTRUMENTATION_CODE: " + code + "\n"

    def test_success_requires_stable_gallery_and_separate_runner(self):
        result = wheel_result(self.output(), 0, "100", "100", 4)
        self.assertEqual(result["gallery_pid_after"], "100")

    def test_terminal_result_is_exact_and_unique(self):
        for text in ("", self.output(code="-10"), self.output(code="0"), self.output() + self.output()):
            with self.subTest(text=text), self.assertRaises(ValueError):
                wheel_result(text, 0, "100", "100", 4)
        with self.assertRaises(ValueError):
            wheel_result(self.output(), 1, "100", "100", 4)

    def test_event_count_type_source_action_and_direction(self):
        for changes in ({"injected": 3}, {"injected": True}, {"passed": "true"}, {"source": "SOURCE_TOUCHSCREEN"}, {"action": "ACTION_MOVE"}, {"vertical_axis": 1}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                wheel_result(self.output(self.record(**changes)), 0, "100", "100", 4)

    def test_missing_restart_and_same_process_fail(self):
        for before, after in (("", ""), ("100", "101"), ("100 101", "100 101"), ("0", "0"), ("200", "200")):
            with self.subTest(before=before), self.assertRaises(ValueError):
                wheel_result(self.output(), 0, before, after, 4)

    def test_coordinates_and_json_shape(self):
        for value in ([], self.record(x=float("nan")), self.record(y=-1), self.record(x=True), self.record(instrumentation_pid=0)):
            with self.subTest(value=value), self.assertRaises(ValueError):
                wheel_result(self.output(value), 0, "100", "100", 4)

    def test_negative_steps_and_invalid_request(self):
        wheel_result(self.output(self.record(vertical_axis=1)), 0, "100", "100", -4)
        with self.assertRaises(ValueError):
            wheel_result(self.output(self.record(vertical_axis=True)), 0, "100", "100", -4)
        for steps in (0, 13, True):
            with self.subTest(steps=steps), self.assertRaises(ValueError):
                wheel_result(self.output(), 0, "100", "100", steps)


if __name__ == "__main__":
    unittest.main()
