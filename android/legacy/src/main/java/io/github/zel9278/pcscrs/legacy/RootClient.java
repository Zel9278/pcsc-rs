package io.github.zel9278.pcscrs.legacy;

import android.content.Context;
import android.os.Build;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.InputStream;
import java.nio.charset.Charset;

final class RootClient {
    static final String DIR = "/data/local/tmp/pcsc-rs";
    private static final String LOOP = "#!/system/bin/sh\n" +
            "cd /data/local/tmp/pcsc-rs || exit 1\n" +
            "echo $$ > loop.pid\n" +
            "unset HOSTNAME\n" +
            "while true; do ./pcsc-rs; sleep 5; done\n";

    private RootClient() { }

    static String abi() {
        String value = Build.CPU_ABI;
        if ("armeabi-v7a".equals(value)) return "armeabi-v7a";
        return "armeabi";
    }

    static File extract(Context context) throws Exception {
        File file = new File(context.getFilesDir(), "pcsc-rs-" + abi());
        InputStream input = context.getAssets().open("bin/" + abi() + "/pcsc-rs");
        java.io.FileOutputStream output = new java.io.FileOutputStream(file);
        byte[] buffer = new byte[8192];
        int count;
        try {
            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
        } finally {
            input.close();
            output.close();
        }
        return file;
    }

    static Result run(String command) {
        Process process = null;
        try {
            process = Runtime.getRuntime().exec(new String[] {"su", "-c", command + " 2>&1"});
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            InputStream input = process.getInputStream();
            byte[] buffer = new byte[4096];
            int count;
            while ((count = input.read(buffer)) != -1) bytes.write(buffer, 0, count);
            int code = process.waitFor();
            return new Result(code, new String(bytes.toByteArray(), Charset.forName("UTF-8")));
        } catch (Exception e) {
            return new Result(-1, e.toString());
        } finally {
            if (process != null) process.destroy();
        }
    }

    static Result status() {
        return run("if pidof pcsc-rs >/dev/null 2>&1; then echo RUNNING; else echo STOPPED; fi; " +
                "tail -n 40 " + DIR + "/pcsc-rs.log 2>/dev/null; true");
    }

    static Result start(Context context, Settings settings) {
        try {
            File binary = extract(context);
            int version = context.getPackageManager().getPackageInfo(context.getPackageName(), 0).versionCode;
            StringBuilder envFile = new StringBuilder();
            envFile.append("PASS='").append(settings.pass).append("'\n");
            if (settings.hostname.trim().length() != 0) {
                envFile.append("HOSTNAME='").append(settings.hostname.trim()).append("'\n");
            }
            if (settings.uri.trim().length() != 0) {
                envFile.append("PCSC_URI='").append(settings.uri.trim()).append("'\n");
            }
            StringBuilder printf = new StringBuilder("printf '%s\\n'");
            for (String line : envFile.toString().split("\\n")) {
                if (line.length() != 0) printf.append(' ').append(quote(line));
            }
            String script = "set -e\n" +
                    "mkdir -p " + DIR + "\nchmod 755 " + DIR + "\ncd " + DIR + "\n" +
                    "if [ ! -x pcsc-rs ] || [ \"$(cat apk-version 2>/dev/null)\" != " + version + " ]; then\n" +
                    "  cp " + quote(binary.getAbsolutePath()) + " pcsc-rs.new\n" +
                    "  chmod 755 pcsc-rs.new\n" +
                    "  mv -f pcsc-rs.new pcsc-rs\n" +
                    "  echo " + version + " > apk-version\nfi\n" +
                    "if [ -f loop.pid ]; then kill $(cat loop.pid) 2>/dev/null || true; fi\n" +
                    "if pidof pcsc-rs >/dev/null 2>&1; then kill $(pidof pcsc-rs) 2>/dev/null || true; fi\n" +
                    printf + " > .env\nchmod 600 .env\n" +
                    "cat > loop.sh <<'PCSC_EOF'\n" + LOOP + "PCSC_EOF\nchmod 755 loop.sh pcsc-rs\n" +
                    "touch pcsc-rs.log\nchown -R 2000:2000 .\n" +
                    "su 2000 <<'PCSC_USER'\n" +
                    "cd /data/local/tmp/pcsc-rs\n" +
                    "(sh ./loop.sh </dev/null >> pcsc-rs.log 2>&1 & echo $! > loop.pid)\n" +
                    "PCSC_USER\n" +
                    "chown 2000:2000 loop.pid\necho started";
            return run(script);
        } catch (Exception e) {
            return new Result(-1, e.toString());
        }
    }

    static Result stop() {
        return run("cd " + DIR + "; " +
                "if [ -f loop.pid ]; then kill $(cat loop.pid) 2>/dev/null || true; rm -f loop.pid; fi; " +
                "if pidof pcsc-rs >/dev/null 2>&1; then kill $(pidof pcsc-rs) 2>/dev/null || true; fi; echo stopped");
    }

    private static String quote(String value) {
        return "'" + value.replace("'", "'\\''") + "'";
    }

    static final class Result {
        final int code;
        final String output;
        Result(int code, String output) { this.code = code; this.output = output; }
        boolean ok() { return code == 0; }
    }
}
