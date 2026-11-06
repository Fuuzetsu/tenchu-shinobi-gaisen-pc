package org.tenchu.port;

import android.app.AlertDialog;
import android.app.NativeActivity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;
import android.widget.EditText;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import org.json.JSONObject;

public final class GameActivity extends NativeActivity {
  static {
    System.loadLibrary("tenchu_pc");
  }

  private static native void submitText(boolean accepted, String text);

  private static native void back();

  private String requestKind, exportPath;
  private AlertDialog graphicsDialog;

  public void preparingGraphics(boolean preparing) {
    runOnUiThread(
        () -> {
          if (preparing && graphicsDialog == null && !isFinishing()) {
            graphicsDialog =
                new AlertDialog.Builder(this)
                    .setTitle("Preparing graphics")
                    .setMessage("Compiling effects for your GPU. This may take a moment.")
                    .setView(new android.widget.ProgressBar(this))
                    .setCancelable(false)
                    .create();
            graphicsDialog.show();
          } else if (!preparing && graphicsDialog != null) {
            graphicsDialog.dismiss();
            graphicsDialog = null;
          }
        });
  }

  public void editText(String value) {
    runOnUiThread(
        () -> {
          EditText field = new EditText(this);
          field.setText(value);
          field.setSelectAllOnFocus(true);
          AlertDialog dialog =
              new AlertDialog.Builder(this)
                  .setTitle("Edit text")
                  .setView(field)
                  .setPositiveButton(
                      "Apply", (d, w) -> submitText(true, field.getText().toString()))
                  .setNegativeButton("Cancel", (d, w) -> submitText(false, ""))
                  .setOnCancelListener(d -> submitText(false, ""))
                  .create();
          dialog
              .getWindow()
              .setSoftInputMode(
                  WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_VISIBLE
                      | WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE);
          dialog.show();
          field.requestFocus();
        });
  }

  public void copyText(String text) {
    runOnUiThread(
        () ->
            getSystemService(ClipboardManager.class)
                .setPrimaryClip(ClipData.newPlainText("Tenchu", text)));
  }

  @Override
  public void onCreate(Bundle state) {
    super.onCreate(state);
    getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
  }

  @Override
  public void onBackPressed() {
    back();
  }

  @Override
  public void onDestroy() {
    super.onDestroy();
    // winit permits one event loop per process; only this activity uses :game.
    android.os.Process.killProcess(android.os.Process.myPid());
  }

  @Override
  public void onWindowFocusChanged(boolean focused) {
    super.onWindowFocusChanged(focused);
    if (focused)
      getWindow()
          .getDecorView()
          .setSystemUiVisibility(
              View.SYSTEM_UI_FLAG_FULLSCREEN
                  | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                  | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);
  }

  public void showError(String error) {
    runOnUiThread(
        () ->
            new AlertDialog.Builder(this)
                .setTitle("Tenchu could not continue")
                .setMessage(error)
                .setPositiveButton("Return to launcher", (d, w) -> finish())
                .setOnCancelListener(d -> finish())
                .show());
  }

  public void fileRequest(String kind, String path, String name, boolean exporting) {
    runOnUiThread(
        () -> {
          if (requestKind != null) return;
          requestKind = kind;
          exportPath = exporting ? path : null;
          Intent intent =
              new Intent(exporting ? Intent.ACTION_CREATE_DOCUMENT : Intent.ACTION_OPEN_DOCUMENT)
                  .setType("application/octet-stream")
                  .addCategory(Intent.CATEGORY_OPENABLE);
          if (exporting) intent.putExtra(Intent.EXTRA_TITLE, name);
          else intent.setType("*/*");
          startActivityForResult(intent, 20);
        });
  }

  @Override
  public void onActivityResult(int request, int result, Intent data) {
    super.onActivityResult(request, result, data);
    if (request != 20 || requestKind == null) return;
    String kind = requestKind, source = exportPath;
    requestKind = null;
    exportPath = null;
    Uri uri = result == RESULT_OK && data != null ? data.getData() : null;
    new Thread(
            () -> {
              JSONObject response = new JSONObject();
              try {
                response.put("kind", kind);
                if (uri != null) {
                  File file =
                      source != null ? new File(source) : new File(getFilesDir(), "import-" + kind);
                  try (InputStream input =
                          source != null
                              ? new FileInputStream(file)
                              : getContentResolver().openInputStream(uri);
                      OutputStream output =
                          source != null
                              ? getContentResolver().openOutputStream(uri, "wt")
                              : new FileOutputStream(file)) {
                    if (input == null || output == null)
                      throw new IOException("Cannot open selected document");
                    byte[] bytes = new byte[256 * 1024];
                    int count;
                    long size = 0;
                    while ((count = input.read(bytes)) != -1) {
                      size += count;
                      if (source == null && size > 1024L * 1024 * 1024)
                        throw new IOException("Document is too large");
                      output.write(bytes, 0, count);
                    }
                  }
                  response.put("path", file.getAbsolutePath());
                }
              } catch (Exception e) {
                try {
                  response.put("error", e.toString());
                } catch (Exception ignored) {
                }
              }
              try {
                File temporary = new File(getFilesDir(), "file-response.tmp");
                Files.write(
                    temporary.toPath(), response.toString().getBytes(StandardCharsets.UTF_8));
                Files.move(
                    temporary.toPath(),
                    new File(getFilesDir(), "file-response.json").toPath(),
                    StandardCopyOption.REPLACE_EXISTING);
              } catch (Exception e) {
                showError(e.toString());
              }
            },
            "tenchu-document")
        .start();
  }
}
