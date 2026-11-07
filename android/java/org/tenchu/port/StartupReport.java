package org.tenchu.port;

import android.app.Activity;
import android.app.ActivityManager;
import android.app.ApplicationExitInfo;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.os.Build;
import android.widget.Toast;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.zip.GZIPOutputStream;
import org.json.JSONArray;
import org.json.JSONObject;

final class StartupReport {
  private static byte[] compressTrace(InputStream input) throws IOException {
    try (InputStream in = input;
        ByteArrayOutputStream out = new ByteArrayOutputStream()) {
      try (GZIPOutputStream gzip = new GZIPOutputStream(out)) {
        byte[] buffer = new byte[8192];
        int total = 0;
        int count;
        while ((count = in.read(buffer)) != -1) {
          total += count;
          if (total > 4 * 1024 * 1024) throw new IOException("Native trace exceeds 4 MiB");
          gzip.write(buffer, 0, count);
          if (out.size() > 128 * 1024)
            throw new IOException("Compressed native trace exceeds clipboard budget");
        }
      }
      if (out.size() > 128 * 1024)
        throw new IOException("Compressed native trace exceeds clipboard budget");
      return out.toByteArray();
    }
  }

  private static JSONObject nativeTrace(ApplicationExitInfo info) throws Exception {
    JSONObject trace = new JSONObject();
    try {
      InputStream input = info.getTraceInputStream();
      if (input == null) {
        trace.put("unavailable", "Android no longer has a trace for this exit");
      } else {
        trace.put("format", "android-tombstone-protobuf");
        trace.put("encoding", "gzip+base64");
        trace.put("data", Base64.getEncoder().encodeToString(compressTrace(input)));
      }
    } catch (Exception e) {
      trace.put("error", e.toString());
    }
    return trace;
  }

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

  static JSONObject collect(Context context, String libraryError) throws Exception {
    JSONObject report = new JSONObject();
    report.put("format", "tenchu-android-startup-report");
    report.put("version", 1);
    report.put("created_unix_ms", System.currentTimeMillis());
    report.put("build", new JSONObject(text(context.getAssets().open("build-info.json"))));
    report.put("android", Build.VERSION.RELEASE);
    report.put("api", Build.VERSION.SDK_INT);
    report.put("manufacturer", Build.MANUFACTURER);
    report.put("model", Build.MODEL);
    report.put("abis", new JSONArray(Build.SUPPORTED_ABIS));
    report.put("library_error", libraryError == null ? JSONObject.NULL : libraryError);
    for (String name : new String[] {"startup-log.txt", "graphics-backend.txt"}) {
      File file = new File(context.getFilesDir(), name);
      if (file.isFile()) report.put(name, text(new FileInputStream(file)));
    }
    if (Build.VERSION.SDK_INT >= 30) {
      JSONArray exits = new JSONArray();
      ActivityManager manager = context.getSystemService(ActivityManager.class);
      boolean traceIncluded = false;
      for (ApplicationExitInfo info : manager.getHistoricalProcessExitReasons(null, 0, 8)) {
        JSONObject exit = new JSONObject();
        exit.put("process", info.getProcessName());
        exit.put("time_ms", info.getTimestamp());
        exit.put("reason", info.getReason());
        exit.put("status", info.getStatus());
        exit.put("description", info.getDescription());
        exit.put("pss_kib", info.getPss());
        exit.put("rss_kib", info.getRss());
        if (!traceIncluded
            && Build.VERSION.SDK_INT >= 31
            && info.getReason() == ApplicationExitInfo.REASON_CRASH_NATIVE
            && (context.getPackageName() + ":game").equals(info.getProcessName())) {
          exit.put("native_trace", nativeTrace(info));
          traceIncluded = true;
        }
        exits.put(exit);
      }
      report.put("exits", exits);
    }
    return report;
  }

  static void copy(Activity activity, String libraryError) {
    new Thread(
            () -> {
              String output;
              try {
                output = collect(activity, libraryError).toString(2);
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
