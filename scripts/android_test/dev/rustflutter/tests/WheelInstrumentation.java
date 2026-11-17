package dev.rustflutter.tests;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.Process;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import org.json.JSONObject;

/** Self-targeted test runner: injects OS events into the foreground ordinary APK.
 * No Activity, model access, native bridge or gallery repackaging is involved. */
public final class WheelInstrumentation extends Instrumentation {
    private Bundle arguments;
    @Override public void onCreate(Bundle args) { arguments = args; start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            float x = Float.parseFloat(arguments.getString("x"));
            float y = Float.parseFloat(arguments.getString("y"));
            int steps = Integer.parseInt(arguments.getString("steps", "4"));
            if (!Float.isFinite(x) || !Float.isFinite(y) || x < 0 || y < 0 || steps == 0 || steps < -12 || steps > 12) {
                throw new IllegalArgumentException("finite screen coordinates and 1..12 signed steps required");
            }
            MotionEvent.PointerProperties properties = new MotionEvent.PointerProperties();
            properties.id = 0;
            properties.toolType = MotionEvent.TOOL_TYPE_MOUSE;
            MotionEvent.PointerCoords coords = new MotionEvent.PointerCoords();
            coords.x = x;
            coords.y = y;
            coords.setAxisValue(MotionEvent.AXIS_VSCROLL, steps > 0 ? -1.0f : 1.0f);
            int count = Math.abs(steps);
            for (int i = 0; i < count; i++) {
                long now = SystemClock.uptimeMillis();
                MotionEvent event = MotionEvent.obtain(now, now, MotionEvent.ACTION_SCROLL, 1,
                    new MotionEvent.PointerProperties[] { properties }, new MotionEvent.PointerCoords[] { coords },
                    0, 0, 1.0f, 1.0f, 0, 0, InputDevice.SOURCE_MOUSE, 0);
                try {
                    if (!getUiAutomation().injectInputEvent(event, true)) {
                        throw new AssertionError("OS rejected mouse scroll packet " + (i + 1));
                    }
                } finally { event.recycle(); }
                // Separate physical-style wheel packets; presentation is checked
                // independently by the screenshot probe, never by this delay.
                if (i + 1 < count) SystemClock.sleep(100);
            }
            JSONObject summary = new JSONObject();
            summary.put("passed", true); summary.put("injected", count);
            summary.put("x", x); summary.put("y", y); summary.put("vertical_axis", steps > 0 ? -1 : 1);
            summary.put("source", "SOURCE_MOUSE"); summary.put("action", "ACTION_SCROLL");
            summary.put("instrumentation_pid", Process.myPid());
            result.putString("summary", summary.toString());
            finish(Activity.RESULT_OK, result);
        } catch (Throwable error) {
            result.putString("failure", android.util.Log.getStackTraceString(error));
            finish(Activity.RESULT_CANCELED, result);
        }
    }
}
