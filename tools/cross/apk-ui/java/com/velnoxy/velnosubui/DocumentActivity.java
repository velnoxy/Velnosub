package com.velnoxy.velnosubui;

import android.app.NativeActivity;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.window.OnBackInvokedDispatcher;
import android.webkit.MimeTypeMap;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.charset.StandardCharsets;
import java.util.Locale;

public final class DocumentActivity extends NativeActivity {
    private static final int OPEN_DOCUMENT = 7301;
    private static final int SAVE_DOCUMENT = 7302;

    static {
        System.loadLibrary("velnosub-abi");
    }

    private static native void documentResult(int status, byte[] path);
    private static native void requestClose();
    private static native void appearanceChanged(int dark, int lightAccent, int darkAccent, int lightSurface, int darkSurface);

    private void publishAppearance() {
        int night = getResources().getConfiguration().uiMode & android.content.res.Configuration.UI_MODE_NIGHT_MASK;
        int dark = night == android.content.res.Configuration.UI_MODE_NIGHT_YES ? 1 : 0;
        if (Build.VERSION.SDK_INT >= 31) {
            appearanceChanged(dark, getColor(android.R.color.system_accent1_700),
                    getColor(android.R.color.system_accent1_200), getColor(android.R.color.system_neutral1_50),
                    getColor(android.R.color.system_neutral1_900));
        } else {
            appearanceChanged(dark, 0, 0, 0, 0);
        }
    }

    @Override protected void onResume() {
        super.onResume();
        publishAppearance();
    }

    @Override public void onConfigurationChanged(android.content.res.Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        publishAppearance();
    }

    private static void report(int status, String path) {
        documentResult(status, path.getBytes(StandardCharsets.UTF_8));
    }

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        if (Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    OnBackInvokedDispatcher.PRIORITY_DEFAULT, DocumentActivity::requestClose);
        }
    }

    @Override public void onBackPressed() {
        requestClose();
    }

    public void closeEditor() {
        runOnUiThread(this::finish);
    }

    private String exportPath;

    public void pickDocument(int kind, byte[] path, byte[] title) {
        runOnUiThread(() -> {
            boolean save = kind == 3;
            Intent intent = new Intent(save ? Intent.ACTION_CREATE_DOCUMENT : Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            if (save) {
                exportPath = new String(path, StandardCharsets.UTF_8);
                String name = new String(title, StandardCharsets.UTF_8);
                intent.setType(typeForExport(name));
                intent.putExtra(Intent.EXTRA_TITLE, name);
            } else {
                intent.setType(kind == 1 ? "video/*" : kind == 2 ? "audio/*" : "*/*");
            }
            try {
                startActivityForResult(intent, save ? SAVE_DOCUMENT : OPEN_DOCUMENT);
            } catch (RuntimeException error) {
                report(-1, "");
            }
        });
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != OPEN_DOCUMENT && request != SAVE_DOCUMENT)
            return;
        Uri uri = data == null ? null : data.getData();
        if (result != RESULT_OK || uri == null) {
            report(2, "");
            return;
        }
        String source = exportPath;
        new Thread(() -> {
            File file = null;
            try {
                if (request == SAVE_DOCUMENT) {
                    if (source == null)
                        throw new IOException("Missing export source");
                    try (OutputStream output = getContentResolver().openOutputStream(uri, "wt")) {
                        if (output == null)
                            throw new IOException("Missing output stream");
                        Files.copy(new File(source).toPath(), output);
                    }
                    report(1, "");
                } else {
                    file = new File(Files.createTempDirectory(getCacheDir().toPath(), "velnosub-import-").toFile(), displayName(uri));
                    try (InputStream input = getContentResolver().openInputStream(uri);
                         OutputStream output = Files.newOutputStream(file.toPath())) {
                        if (input == null)
                            throw new IOException("Missing input stream");
                        byte[] buffer = new byte[64 * 1024];
                        int count;
                        while ((count = input.read(buffer)) != -1)
                            output.write(buffer, 0, count);
                    }
                    report(1, file.getAbsolutePath());
                }
            } catch (IOException | RuntimeException error) {
                if (file != null) {
                    file.delete();
                    file.getParentFile().delete();
                }
                report(-1, "");
            }
        }, "velnosub-document").start();
    }

    private static String typeForExport(String name) {
        int dot = name.lastIndexOf('.');
        String extension = dot < 0 ? "" : name.substring(dot + 1).toLowerCase(Locale.ROOT);
        String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
        return type == null ? "application/octet-stream" : type;
    }

    private String displayName(Uri uri) {
        try (android.database.Cursor cursor = getContentResolver().query(
                uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                String name = cursor.getString(0);
                if (name != null) {
                    name = name.replace('/', '_').replace((char) 92, '_').replaceAll("[\\p{Cntrl}]", "_");
                    // Older exports could have the provider's collision number after the extension.
                    name = name.replaceFirst("(?i)^(.+)\\.(ass|ssa|srt) (\\(\\d+\\))$", "$1 $3.$2");
                    if (!name.isEmpty() && !name.equals(".") && !name.equals("..") &&
                            name.getBytes(StandardCharsets.UTF_8).length <= 240)
                        return name;
                    int dot = name.lastIndexOf('.');
                    String suffix = dot < 0 ? "" : name.substring(dot).toLowerCase(Locale.ROOT);
                    if (suffix.matches("\\.[a-z0-9]{1,8}"))
                        return "belge" + suffix;
                }
            }
        }
        return "belge.bin";
    }
}
