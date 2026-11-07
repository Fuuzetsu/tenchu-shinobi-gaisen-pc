package org.tenchu.port;

import android.app.Activity;
import android.app.ActivityManager;
import android.app.ApplicationExitInfo;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.os.Build;
import android.widget.Toast;
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.json.JSONArray;
import org.json.JSONObject;

final class StartupReport {
  private static String text(InputStream input) throws IOException {
    try (InputStream in = input;
        ByteArrayOutputStream out = new ByteArrayOutputStream()) {
      byte[] buffer = new byte[4096];
      while (out.size() < 128 * 1024) {
        int count = in.read(buffer, 0, Math.min(buffer.length, 128 * 1024 - out.size()));
        if (count < 0) break;
        out.write(buffer, 0, count);
      }
      return new String(out.toByteArray(), StandardCharsets.UTF_8);
    }
  }

  static void copy(Activity activity, String libraryError) {
    new Thread(
            () -> {
              String output;
              try {
                JSONObject report = new JSONObject();
                report.put("format", "tenchu-android-startup-report");
                report.put("version", 1);
                report.put(
                    "build", new JSONObject(text(activity.getAssets().open("build-info.json"))));
                report.put("android", Build.VERSION.RELEASE);
                report.put("api", Build.VERSION.SDK_INT);
                report.put("manufacturer", Build.MANUFACTURER);
                report.put("model", Build.MODEL);
                report.put("abis", new JSONArray(Build.SUPPORTED_ABIS));
                report.put("library_error", libraryError == null ? JSONObject.NULL : libraryError);
                for (String name : new String[] {"startup-log.txt", "graphics-backend.txt"}) {
                  File file = new File(activity.getFilesDir(), name);
                  if (file.isFile()) report.put(name, text(new FileInputStream(file)));
                }
                if (Build.VERSION.SDK_INT >= 30) {
                  JSONArray exits = new JSONArray();
                  ActivityManager manager = activity.getSystemService(ActivityManager.class);
                  for (ApplicationExitInfo info :
                      manager.getHistoricalProcessExitReasons(null, 0, 8)) {
                    JSONObject exit = new JSONObject();
                    exit.put("process", info.getProcessName());
                    exit.put("time_ms", info.getTimestamp());
                    exit.put("reason", info.getReason());
                    exit.put("status", info.getStatus());
                    exit.put("description", info.getDescription());
                    exit.put("pss_kib", info.getPss());
                    exit.put("rss_kib", info.getRss());
                    exits.put(exit);
                  }
                  report.put("exits", exits);
                }
                output = report.toString(2);
              } catch (Exception e) {
                output = "Could not collect startup report: " + e;
              }
              final String result = output;
              activity.runOnUiThread(
                  () -> {
                    activity
                        .getSystemService(ClipboardManager.class)
                        .setPrimaryClip(ClipData.newPlainText("Tenchu startup report", result));
                    Toast.makeText(activity, "Startup report copied", Toast.LENGTH_SHORT).show();
                  });
            },
            "tenchu-startup-report")
        .start();
  }
}
