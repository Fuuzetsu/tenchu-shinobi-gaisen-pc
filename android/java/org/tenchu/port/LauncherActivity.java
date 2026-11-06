package org.tenchu.port;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.widget.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.*;
import java.util.concurrent.CompletableFuture;

public final class LauncherActivity extends Activity {
  static {
    System.loadLibrary("tenchu_pc");
  }

  private static native String validateDisc(String path);

  private static native String[] discFiles(String path) throws IOException;

  private static native String prepareArtwork(String root, String path);

  private LinearLayout content;
  private TextView status;
  private boolean busy;
  private static final int DISC = 10, ARTWORK = 11;

  @Override
  public void onCreate(Bundle state) {
    super.onCreate(state);
    ScrollView scroll = new ScrollView(this);
    content = new LinearLayout(this);
    content.setOrientation(LinearLayout.VERTICAL);
    int pad = dp(24);
    content.setPadding(pad, pad, pad, pad);
    scroll.addView(content);
    setContentView(scroll);
    label("Tenchu: Shinobi Gaisen", 26);
    label(
        "Native Android port · bring your own Shinobi Gaisen disc (SLPS-019.01). No game assets are"
            + " included.",
        16);
    status = label("", 16);
    button(
        "Play",
        () -> {
          if (!new File(getFilesDir(), "disc-path.txt").isFile()) {
            status.setText("Import your disc folder first.");
            return;
          }
          startActivity(
              new Intent(this, GameActivity.class)
                  .addFlags(
                      Intent.FLAG_ACTIVITY_REORDER_TO_FRONT | Intent.FLAG_ACTIVITY_SINGLE_TOP));
        });
    button(
        "Import disc folder…",
        () -> {
          status.setText(
              "Choose a folder containing one .cue and its .bin files. On Android, put these in a"
                  + " subfolder of Downloads, not the Downloads root.");
          startActivityForResult(new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE), DISC);
        });
    button(
        "Import HD artwork (.tar.zst)…",
        () ->
            startActivityForResult(
                new Intent(Intent.ACTION_OPEN_DOCUMENT)
                    .setType("*/*")
                    .addCategory(Intent.CATEGORY_OPENABLE),
                ARTWORK));
    label(
        "Touch: hold Move and tap actions with another finger. Settings opens the F1 menu. Pause"
            + " and Back are always on screen. Camera-relative controls add a right camera stick."
            + " Importing copies files into this app; your originals are unchanged.",
        16);
    refresh();
  }

  @Override
  public void onResume() {
    super.onResume();
    if (!busy) refresh();
  }

  private void refresh() {
    File path = new File(getFilesDir(), "disc-path.txt");
    status.setText(
        path.isFile()
            ? "Disc imported. "
                + (new File(getFilesDir(), "tenchu-hd.tar.zst").isFile()
                    ? "HD artwork installed."
                    : "Original artwork; HD pack is optional.")
            : "No disc imported yet.");
  }

  private int dp(int n) {
    return Math.round(n * getResources().getDisplayMetrics().density);
  }

  private TextView label(String text, int size) {
    TextView v = new TextView(this);
    v.setText(text);
    v.setTextSize(size);
    v.setPadding(0, dp(6), 0, dp(6));
    content.addView(v);
    return v;
  }

  private void button(String text, Runnable action) {
    Button b = new Button(this);
    b.setText(text);
    b.setOnClickListener(
        v -> {
          if (!busy) action.run();
        });
    content.addView(b);
  }

  private interface Job {
    void run() throws Exception;
  }

  private void work(Job job) {
    busy = true;
    status.setText("Importing… Keep this screen open. Large files can take a little while.");
    new Thread(
            () -> {
              String error = null;
              try {
                job.run();
              } catch (Exception e) {
                android.util.Log.e("TenchuImport", "Import failed", e);
                error = e.getMessage() == null ? e.toString() : e.getMessage();
              }
              final String message = error;
              runOnUiThread(
                  () -> {
                    busy = false;
                    if (message == null) refresh();
                    else
                      new AlertDialog.Builder(this)
                          .setTitle("Import failed")
                          .setMessage(message)
                          .setNeutralButton(
                              "Copy error",
                              (d, w) ->
                                  getSystemService(android.content.ClipboardManager.class)
                                      .setPrimaryClip(
                                          android.content.ClipData.newPlainText(
                                              "Tenchu import error", message)))
                          .setPositiveButton("OK", null)
                          .setOnDismissListener(d -> refresh())
                          .show();
                  });
            },
            "tenchu-import")
        .start();
  }

  @Override
  public void onActivityResult(int request, int result, Intent data) {
    super.onActivityResult(request, result, data);
    if (result != RESULT_OK || data == null || data.getData() == null) return;
    Uri uri = data.getData();
    if (request == DISC) work(() -> importDisc(uri));
    if (request == ARTWORK)
      work(
          () -> {
            File temporary = new File(getFilesDir(), "artwork-import.tar.zst");
            try {
              copy(uri, temporary, 512L * 1024 * 1024);
              runOnUiThread(() -> status.setText("Checking and unpacking artwork…"));
              String error =
                  prepareArtwork(getFilesDir().getAbsolutePath(), temporary.getAbsolutePath());
              if (error != null) throw new IOException(error);
              Files.move(
                  temporary.toPath(),
                  new File(getFilesDir(), "tenchu-hd.tar.zst").toPath(),
                  StandardCopyOption.REPLACE_EXISTING);
            } finally {
              temporary.delete();
            }
          });
  }

  private void importDisc(Uri tree) throws Exception {
    Uri children =
        DocumentsContract.buildChildDocumentsUriUsingTree(
            tree, DocumentsContract.getTreeDocumentId(tree));
    Map<String, Uri> files = new LinkedHashMap<>();
    String cue = null;
    try (Cursor cursor =
        getContentResolver()
            .query(
                children,
                new String[] {
                  DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                  DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                  DocumentsContract.Document.COLUMN_MIME_TYPE
                },
                null,
                null,
                null)) {
      if (cursor == null) throw new IOException("Cannot read this folder");
      while (cursor.moveToNext()) {
        if (DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(2))) continue;
        String name = cursor.getString(1), lower = name.toLowerCase(Locale.ROOT);
        if (name.contains("/") || name.contains("\\") || name.equals("..")) continue;
        if (lower.endsWith(".cue")) {
          if (cue != null)
            throw new IOException(
                "Choose a folder containing just one CUE file and its BIN files.");
          cue = name;
        }
        if (files.put(name, DocumentsContract.buildDocumentUriUsingTree(tree, cursor.getString(0)))
            != null) throw new IOException("This folder contains duplicate filenames: " + name);
      }
    }
    if (cue == null)
      throw new IOException(
          "No .cue found. Place the CUE and BIN files together in a folder, then select that"
              + " folder.");
    File folder = new File(getFilesDir(), "disc-" + System.currentTimeMillis());
    if (!folder.mkdir()) throw new IOException("Cannot create disc directory");
    boolean complete = false;
    try {
      File disc = new File(folder, cue);
      long total = copy(files.remove(cue), disc, 1024 * 1024);
      String[] references = discFiles(disc.getAbsolutePath());
      Map<String, Uri> tracks = new LinkedHashMap<>();
      for (String name : references) {
        if (name.contains("/") || name.contains("\\") || name.equals("..") || name.equals(cue))
          throw new IOException("CUE track names must be filenames in this folder: " + name);
        if (files.containsKey(name)) tracks.put(name, files.remove(name));
      }
      for (String name : references) {
        if (tracks.containsKey(name)) continue;
        String match = matchingFile(name, files.keySet());
        if (match == null) match = chooseTrack(name, files.keySet());
        tracks.put(name, files.remove(match));
      }
      for (Map.Entry<String, Uri> entry : tracks.entrySet()) {
        String name = entry.getKey();
        runOnUiThread(() -> status.setText("Importing " + name + "…"));
        total += copy(entry.getValue(), new File(folder, name), 1024L * 1024 * 1024 - total);
      }
      runOnUiThread(() -> status.setText("Checking the disc version and game files…"));
      String error = validateDisc(disc.getAbsolutePath());
      if (error != null) throw new IOException(error);
      File saved = new File(getFilesDir(), "disc-path.txt");
      String previous =
          saved.isFile()
              ? new String(Files.readAllBytes(saved.toPath()), StandardCharsets.UTF_8)
              : "";
      File temporary = new File(getFilesDir(), "disc-path.tmp");
      Files.write(temporary.toPath(), disc.getAbsolutePath().getBytes(StandardCharsets.UTF_8));
      Files.move(temporary.toPath(), saved.toPath(), StandardCopyOption.REPLACE_EXISTING);
      complete = true;
      File old = new File(previous).getParentFile();
      if (old != null
          && old.getParentFile().equals(getFilesDir())
          && old.getName().startsWith("disc-")
          && !old.equals(folder)) remove(old);
    } finally {
      if (!complete) remove(folder);
    }
  }

  private static String matchingFile(String expected, Set<String> names) {
    // Firefox for Android replaces commas in downloaded filenames with underscores.
    String key = expected.replace(',', '_');
    String match = null;
    for (String name : names) {
      if (!key.equalsIgnoreCase(name.replace(',', '_'))) continue;
      if (match != null) return null;
      match = name;
    }
    return match;
  }

  private String chooseTrack(String expected, Set<String> names) throws Exception {
    if (names.isEmpty())
      throw new IOException(
          "Missing BIN file:\n"
              + expected
              + "\n\nKeep the CUE and all its BIN tracks in one folder.");
    String[] choices = names.toArray(new String[0]);
    CompletableFuture<String> answer = new CompletableFuture<>();
    runOnUiThread(
        () -> {
          TextView title = new TextView(this);
          title.setText("Choose the renamed BIN for:\n" + expected);
          title.setTextSize(18);
          title.setPadding(dp(24), dp(18), dp(24), dp(12));
          new AlertDialog.Builder(this)
              .setCustomTitle(title)
              .setAdapter(
                  new ArrayAdapter<>(this, android.R.layout.simple_list_item_1, choices),
                  (d, index) -> answer.complete(choices[index]))
              .setNegativeButton("Cancel", (d, w) -> answer.complete(null))
              .setOnCancelListener(d -> answer.complete(null))
              .show();
        });
    String choice = answer.get();
    if (choice == null) throw new IOException("Disc import cancelled.");
    return choice;
  }

  private long copy(Uri source, File destination, long limit) throws IOException {
    try (InputStream input = getContentResolver().openInputStream(source);
        OutputStream output = new FileOutputStream(destination)) {
      if (input == null) throw new IOException("Cannot open selected file");
      byte[] buffer = new byte[256 * 1024];
      long total = 0;
      int count;
      while ((count = input.read(buffer)) != -1) {
        total += count;
        if (total > limit) throw new IOException("Selected files exceed the supported size");
        output.write(buffer, 0, count);
      }
      return total;
    }
  }

  private static void remove(File directory) {
    File[] files = directory.listFiles();
    if (files != null)
      for (File f : files)
        if (f.isDirectory()) remove(f);
        else f.delete();
    directory.delete();
  }
}
