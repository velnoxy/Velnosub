package com.velnoxy.velnosub;

import android.app.Activity;
import android.content.Context;
import android.os.Bundle;
import android.os.Build;
import android.text.method.ScrollingMovementMethod;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.BufferedOutputStream;
import java.util.ArrayList;

/**
 * velnosub alpha — mobil Aegisub çekirdeği duman köşesi.
 *
 * Çalışma düzeni: assets/velnosub/* ilk açılışta filesDir'e kopyalanır;
 * "Pump Duman" düğmesi nativeLibraryDir'deki libvelnosub_automation.so
 * (velnosub-automation bionic aarch64 ELF'ı) adında pump koşumu başlatır:
 * demoların üzerinden (bacaltı örnek: ass-format-tests.ass üzerinde
 * "Dolaş dene") geçer ve çıkışı TextView'a yazar.
 */
public class MainActivity extends Activity {
    private TextView out;
    private File pkgDir;

    @Override protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        LinearLayout ana = new LinearLayout(this);
        ana.setOrientation(LinearLayout.VERTICAL);

        Button kopyala = new Button(this);
        kopyala.setText("Paketi kopyala");
        Button pump = new Button(this);
        pump.setText("Pump duman (Dolaş)");
        Button kara = new Button(this);
        kara.setText("Pump duman (Kara)");
        out = new TextView(this);
        out.setPadding(8, 8, 8, 8);
        out.setTypeface(null, 3);
        out.setTextSize(12);
        out.setMovementMethod(new ScrollingMovementMethod());

        ana.addView(kopyala);
        ana.addView(pump);
        ana.addView(kara);
        final ScrollView scrol = new ScrollView(this);
        scrol.addView(out);
        ana.addView(scrol);

        setContentView(ana);

        pkgDir = new File(getFilesDir(), "velnosub");

        kopyala.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                try {
                    log("paket kopyalanıyor: " + pkgDir.getPath());
                    copyAssetDir("velnosub", pkgDir);
                    log("kopya bitti: " + fileCount(pkgDir) + " dosya");
                } catch (Exception e) {
                    log("KOPYA HATASI: " + e);
                }
            }
        });

        pump.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                kos("ass-format-tests.ass", "Dolaş dene", "--sel", "3");
            }
        });

        kara.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                kos("karaskel-dene.ass", "Kara dene", "--sel", "1", "--sel", "2", "--sel", "3", "--sel", "4", "--sel", "5");
            }
        });
        Button kk = new Button(this);
        kk.setText("Pump duman (Karaskel)");
        kk.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                kos("karaskel-dene.ass", "Karaskel dene", "--sel", "1");
            }
        });
        ana.addView(kk, 3); // scrol'ün ÖNCESİNDE kalması şart — yoksa düğme ekran altında kayar
    }

    private void log(final String s) {
        runOnUiThread(new Runnable() {
            public void run() {
                out.append(s + "\n");
                android.util.Log.i("velnosub", s);
            }
        });
    }

    private void kos(String ass, String makro, String... selArgs) {
        ArrayList<String> cmd = new ArrayList<String>();
        cmd.add(new File(getApplicationInfo().nativeLibraryDir,
                "libvelnosub_automation.so").getPath());
        cmd.add(new File(pkgDir, "share/lua/velnosub-demo-macro.lua").getPath());
        cmd.add("--ass");
        cmd.add(new File(pkgDir, ass).getPath());
        cmd.add("--macro");
        cmd.add(makro);
        for (String s : selArgs) cmd.add(s);
        kosCmd(cmd, makro);
    }

    private void kosCmd(final ArrayList<String> cmd, final String etiket) {
        new Thread(new Runnable() {
            public void run() {
                try {
                    log("--- " + etiket + " koşumu ---");
                    File lib = new File(getApplicationInfo().nativeLibraryDir);
                    File shim = new File(pkgDir, "share/lua/velnosub-aegisub.lua");
                    File font = new File(pkgDir, "DejaVuSans.ttf");
                    File conf = new File(pkgDir, "default_config.json");
                    File inc = new File(pkgDir, "share/lua/inc");
                    ProcessBuilder pb = new ProcessBuilder(cmd);
                    pb.redirectErrorStream(true);
                    pb.directory(pkgDir);
                    pb.environment().put("LD_LIBRARY_PATH", lib.getPath());
                    pb.environment().put("VELNOSUB_AEGISHIM", shim.getPath());
                    pb.environment().put("VELNOSUB_AEGIINC", inc.getPath());
                    pb.environment().put("VELNOSUB_CONFIG_JSON", conf.getPath());
                    pb.environment().put("VELNOSUB_FONT", font.getPath());
                    pb.redirectInput(new File("/dev/null"));
                    Process pr = pb.start();
                    InputStream is = pr.getInputStream();
                    byte[] en = new byte[8192];
                    StringBuilder sb = new StringBuilder();
                    int n;
                    while ((n = is.read(en)) > 0) {
                        sb.append(new String(en, 0, n, "UTF-8"));
                    }
                    int rc = pr.waitFor();
                    log("RC=" + rc);
                    log(sb.toString());
                    log("--- " + etiket + " bitti ---");
                } catch (Exception e) {
                    log("KOŞUM HATASI: " + e);
                }
            }
        }).start();
    }

    private void copyAssetDir(String ad, File hedef) throws Exception {
        String[] liste = getAssets().list(ad);
        if (liste == null || liste.length == 0) {
            copyAssetFile(ad, hedef);
            return;
        }
        hedef.mkdirs();
        for (String g : liste) {
            copyAssetDir(ad + "/" + g, new File(hedef, g));
        }
    }

    private void copyAssetFile(String ad, File hedef) throws Exception {
        InputStream is = getAssets().open(ad);
        OutputStream os = new BufferedOutputStream(new FileOutputStream(hedef));
        byte[] buf = new byte[32768];
        int n;
        while ((n = is.read(buf)) > 0) os.write(buf, 0, n);
        os.close();
        is.close();
        hedef.setExecutable(true, false);
    }

    private int fileCount(File d) {
        int say = 0;
        File[] l = d.listFiles();
        if (l != null) {
            for (File f : l) say += f.isDirectory() ? fileCount(f) : 1;
        }
        return say;
    }
}
