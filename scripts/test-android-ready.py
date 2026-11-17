#!/usr/bin/env python3
"""GPU-free regression tests for cold-start/resume readiness gating."""
import unittest
from android_ready import focused_presentation, paused_since


def resume(t=1, host=1):
    return f"11-17 12:00:0{t}.000 I RustStdoutStderr: rust-flutter android host={host} pid=100 resumed"


def present(t=2, host=1, focused=True, scale=2.625, top=128):
    return f"11-17 12:00:0{t}.000 I RustStdoutStderr: rust-flutter android host={host} pid=100 presented 1080x2400 scale={scale} safe_top_px={top} focused={str(focused).lower()} keyboard=Hidden"


class Readiness(unittest.TestCase):
    def ready(self, lines, before=()):
        return focused_presentation(set(before), "\n".join(lines), 2.625, 128)

    def test_cold_start_focused_present(self):
        self.assertTrue(self.ready([resume(), present()]))

    def test_old_geometry_and_old_resume_cannot_satisfy_new_launch(self):
        old = [resume(), present()]
        self.assertFalse(self.ready(old, old))
        self.assertFalse(self.ready(old + [present(4)], old))

    def test_resumed_but_unfocused_is_not_input_ready(self):
        self.assertFalse(self.ready([resume(), present(focused=False)]))
        self.assertTrue(self.ready([resume(), present(focused=False), present(3)]))
        self.assertFalse(self.ready([resume(), present(), present(3, focused=False)]))

    def test_lifecycle_loss_invalidates_previously_ready_frame(self):
        for event in ["paused", "surface-created", "surface-destroyed", "destroyed", "returned", "focus-lost"]:
            line = f"11-17 12:00:03.000 I RustStdoutStderr: rust-flutter android host=1 pid=100 {event}"
            self.assertFalse(self.ready([resume(), present(), line]))
            self.assertTrue(self.ready([resume(), present(), line, resume(4), present(5)]))

    def test_presentation_before_fresh_resume_does_not_count(self):
        self.assertFalse(self.ready([present(), resume(3)]))

    def test_expected_density_and_insets_are_required(self):
        self.assertFalse(self.ready([resume(), present(scale=1.0)]))
        self.assertFalse(self.ready([resume(), present(top=0)]))

    def test_latest_resume_host_and_new_timestamp_are_required(self):
        self.assertFalse(self.ready([resume(), present(), resume(3, 2), present(4)]))
        self.assertTrue(self.ready([resume(), present(), resume(3, 2), present(4, 2)]))
        old = [resume(), present()]
        self.assertTrue(self.ready(old + [resume(3), present(4)], old))

    def test_background_requires_fresh_pause_not_interrupted_by_resume(self):
        pause = resume().replace("resumed", "paused")
        self.assertFalse(paused_since({pause}, pause))
        self.assertTrue(paused_since(set(), pause))
        self.assertFalse(paused_since(set(), pause + "\n" + resume(2)))
        self.assertFalse(paused_since(set(), pause.replace("paused", "destroyed")))


if __name__ == "__main__":
    unittest.main()
