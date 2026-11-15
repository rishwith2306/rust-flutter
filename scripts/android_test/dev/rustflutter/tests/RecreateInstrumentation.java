package dev.rustflutter.tests;

import android.app.Activity;
import android.app.Application;
import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.os.Bundle;
import android.os.Process;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.util.concurrent.atomic.AtomicInteger;

/** A separate instrumentation APK; no recreation backdoor is added to the app. */
public final class RecreateInstrumentation extends Instrumentation {
    private Bundle arguments;
    private volatile Activity resumed;
    private final AtomicInteger destroyed = new AtomicInteger();
    private float scale;
    private int top;
    private final Application.ActivityLifecycleCallbacks callbacks = new Application.ActivityLifecycleCallbacks() {
        public void onActivityCreated(Activity a, Bundle state) { }
        public void onActivityStarted(Activity a) { }
        public void onActivityResumed(Activity a) { resumed = a; }
        public void onActivityPaused(Activity a) { }
        public void onActivityStopped(Activity a) { }
        public void onActivitySaveInstanceState(Activity a, Bundle state) { }
        public void onActivityDestroyed(Activity a) { destroyed.incrementAndGet(); }
    };
    @Override public void onCreate(Bundle args) { arguments = args; start(); }

    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            scale = Float.parseFloat(arguments.getString("scale", "0"));
            top = Integer.parseInt(arguments.getString("top", "-1"));
            int cycles = Integer.parseInt(arguments.getString("cycles", "3"));
            Application application = (Application) getTargetContext().getApplicationContext();
            application.registerActivityLifecycleCallbacks(callbacks);
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("dev.rustflutter.gallery", "android.app.NativeActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            Activity initial = startActivitySync(intent);
            long deadline = SystemClock.elapsedRealtime() + 30000;
            while (resumed == null && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50);
            if (resumed != initial) throw new AssertionError("initial Activity did not resume");
            SystemClock.sleep(2000);
            runOnMainSync(() -> {
                if (scale == 0) scale = initial.getResources().getDisplayMetrics().density;
                if (top < 0) {
                    android.view.WindowInsets insets = initial.getWindow().getDecorView().getRootWindowInsets();
                    if (insets == null) throw new AssertionError("WindowInsets are not ready");
                    if (android.os.Build.VERSION.SDK_INT >= 30) {
                        top = insets.getInsets(android.view.WindowInsets.Type.systemBars() | android.view.WindowInsets.Type.displayCutout()).top;
                    } else { top = insets.getSystemWindowInsetTop(); }
                }
            });
            JSONObject before = state();
            if (!"home".equals(before.getString("route"))) throw new AssertionError("return to gallery home before testing");
            increment(before); // Prove the initial surface is interactive as well.
            JSONArray samples = new JSONArray();
            for (int cycle = 0; cycle < cycles; cycle++) {
                before = state();
                final Activity old = resumed;
                int oldDestroyed = destroyed.get();
                runOnMainSync(() -> old.recreate());
                deadline = SystemClock.elapsedRealtime() + 30000;
                while ((resumed == old || destroyed.get() == oldDestroyed) && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50);
                if (resumed == old || destroyed.get() <= oldDestroyed) throw new AssertionError("Activity.recreate did not complete destroy/resume handshake");
                SystemClock.sleep(2000);
                JSONObject restored = state();
                if (!restored.toString().equals(before.toString())) throw new AssertionError("state changed during recreation: " + restored);
                JSONObject after = increment(before);
                JSONObject sample = new JSONObject();
                sample.put("cycle", cycle + 1); sample.put("pid_before", Process.myPid()); sample.put("pid_after", Process.myPid());
                sample.put("activity_before", System.identityHashCode(old)); sample.put("activity_after", System.identityHashCode(resumed));
                sample.put("destroy_callbacks", destroyed.get()); sample.put("state_after_touch", after);
                samples.put(sample);
            }
            // Model writes precede GPU presentation and the label is constructed
            // before the button changes the counter. Wait for the following frame.
            SystemClock.sleep(2000);
            Bitmap bitmap = getUiAutomation().takeScreenshot();
            if (bitmap == null) throw new AssertionError("could not capture recreated window");
            try (FileOutputStream file = new FileOutputStream(new File(getTargetContext().getCacheDir(), "recreation-final.png"))) {
                if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, file)) throw new AssertionError("screenshot encoding failed");
            }
            bitmap.recycle();
            // Also exercise finishing the last Activity, which must release its Rust loop.
            int oldDestroyed = destroyed.get();
            runOnMainSync(() -> resumed.finish());
            deadline = SystemClock.elapsedRealtime() + 30000;
            while (destroyed.get() == oldDestroyed && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50);
            if (destroyed.get() == oldDestroyed) throw new AssertionError("Activity.finish did not destroy the last host");
            application.unregisterActivityLifecycleCallbacks(callbacks);
            JSONObject summary = new JSONObject();
            summary.put("scale", scale); summary.put("top_inset_px", top);
            summary.put("passed", true); summary.put("pid", Process.myPid()); summary.put("cycles", samples);
            summary.put("final_activity_finished", true); summary.put("destroy_callbacks", destroyed.get());
            result.putString("summary", summary.toString());
            finish(Activity.RESULT_OK, result);
        } catch (Throwable error) {
            result.putString("failure", android.util.Log.getStackTraceString(error));
            finish(Activity.RESULT_CANCELED, result);
        }
    }
    private JSONObject state() throws Exception {
        File file = new File(getTargetContext().getFilesDir(), "gallery.json");
        long deadline = SystemClock.elapsedRealtime() + 20000;
        while (!file.exists() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50);
        try (FileInputStream input = new FileInputStream(file); ByteArrayOutputStream bytes = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[4096]; int n;
            while ((n = input.read(buffer)) != -1) bytes.write(buffer, 0, n);
            return new JSONObject(new String(bytes.toByteArray(), "UTF-8")).getJSONObject("data");
        }
    }
    private JSONObject increment(JSONObject before) throws Exception {
        float x = 270 * scale;
        float y = (before.getString("name").isEmpty() ? 187 : 205) * scale + top;
        long down = SystemClock.uptimeMillis();
        MotionEvent press = MotionEvent.obtain(down, down, MotionEvent.ACTION_DOWN, x, y, 0);
        press.setSource(InputDevice.SOURCE_TOUCHSCREEN); sendPointerSync(press); press.recycle();
        SystemClock.sleep(30);
        MotionEvent release = MotionEvent.obtain(down, SystemClock.uptimeMillis(), MotionEvent.ACTION_UP, x, y, 0);
        release.setSource(InputDevice.SOURCE_TOUCHSCREEN); sendPointerSync(release); release.recycle();
        long deadline = SystemClock.elapsedRealtime() + 20000;
        JSONObject value = null;
        while (SystemClock.elapsedRealtime() < deadline) {
            value = state();
            if (value.getInt("count") == before.getInt("count") + 1 && value.getString("name").equals(before.getString("name"))) return value;
            SystemClock.sleep(50);
        }
        throw new AssertionError("recreated UI did not increment saved counter: " + value);
    }
}
