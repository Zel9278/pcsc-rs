package io.github.zel9278.pcscrs.legacy;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

public final class MainActivity extends Activity {
    private final Handler handler = new Handler();
    private EditText pass;
    private EditText hostname;
    private EditText uri;
    private CheckBox boot;
    private TextView status;
    private TextView log;

    private final Runnable refresh = new Runnable() {
        public void run() {
            work(new Work() {
                public RootClient.Result execute() { return RootClient.status(); }
                public void done(RootClient.Result result) {
                    if (!result.ok()) {
                        status.setText("Root access required\n" + result.output);
                    } else {
                        String output = result.output;
                        status.setText(output.startsWith("RUNNING") ? "Running" : "Stopped");
                        int start = output.indexOf('\n');
                        log.setText(start < 0 ? "" : output.substring(start + 1));
                    }
                    handler.postDelayed(refresh, 4000);
                }
            });
        }
    };

    public void onCreate(Bundle state) {
        super.onCreate(state);
        buildUi();
        Settings settings = Settings.load(this);
        pass.setText(settings.pass);
        hostname.setText(settings.hostname);
        uri.setText(settings.uri);
        boot.setChecked(settings.startOnBoot);
    }

    public void onResume() {
        super.onResume();
        handler.removeCallbacks(refresh);
        handler.post(refresh);
    }

    public void onPause() {
        handler.removeCallbacks(refresh);
        super.onPause();
    }

    private void buildUi() {
        ScrollView scroll = new ScrollView(this);
        LinearLayout page = new LinearLayout(this);
        page.setOrientation(LinearLayout.VERTICAL);
        int pad = dp(16);
        page.setPadding(pad, pad, pad, pad);
        scroll.addView(page);
        setContentView(scroll);

        addText(page, "pcsc-rs ﾂｷ Android 2.3+");
        addText(page, "Requires root. Uses the ARMv6 or ARMv7 client bundled in this APK.");
        addText(page, "Access");
        addText(page, "Root access runs the client as the shell user when supported.");
        addText(page, "Status");
        status = addText(page, "Checking...");

        LinearLayout buttons = new LinearLayout(this);
        buttons.setOrientation(LinearLayout.HORIZONTAL);
        Button start = new Button(this);
        start.setText("Start / restart");
        buttons.addView(start, new LinearLayout.LayoutParams(0, -2, 1));
        Button stop = new Button(this);
        stop.setText("Stop");
        buttons.addView(stop, new LinearLayout.LayoutParams(0, -2, 1));
        page.addView(buttons);

        addText(page, "Settings");
        pass = field(page, "Password", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        hostname = field(page, "Device name (optional)", InputType.TYPE_CLASS_TEXT);
        uri = field(page, "Server URI (optional)", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        boot = new CheckBox(this);
        boot.setText("Start after reboot");
        page.addView(boot);
        addText(page, "The app needs root access. ADB pairing and Shizuku are not available on Android 2.3.");
        addText(page, "Log");
        log = addText(page, "");

        start.setOnClickListener(new View.OnClickListener() {
            public void onClick(View view) { startClient(); }
        });
        stop.setOnClickListener(new View.OnClickListener() {
            public void onClick(View view) {
                saveSettings();
                work(new Work() {
                    public RootClient.Result execute() { return RootClient.stop(); }
                    public void done(RootClient.Result result) { show(result); }
                });
            }
        });
    }

    private void startClient() {
        final Settings settings = saveSettings();
        if (!settings.valid()) {
            Toast.makeText(this, "Values cannot contain quotes or line breaks", Toast.LENGTH_LONG).show();
            return;
        }
        if (settings.pass.length() == 0) {
            Toast.makeText(this, "Enter a password first", Toast.LENGTH_LONG).show();
            return;
        }
        work(new Work() {
            public RootClient.Result execute() { return RootClient.start(MainActivity.this, settings); }
            public void done(RootClient.Result result) { show(result); }
        });
    }

    private Settings saveSettings() {
        Settings settings = new Settings(pass.getText().toString(), hostname.getText().toString(),
                uri.getText().toString(), boot.isChecked());
        settings.save(this);
        return settings;
    }

    private void work(final Work action) {
        new Thread(new Runnable() {
            public void run() {
                final RootClient.Result result = action.execute();
                runOnUiThread(new Runnable() { public void run() { action.done(result); } });
            }
        }).start();
    }

    private void show(RootClient.Result result) {
        String value = result.ok() ? result.output : "Failed (" + result.code + "): " + result.output;
        Toast.makeText(this, value.length() > 160 ? value.substring(0, 160) : value, Toast.LENGTH_LONG).show();
    }

    private EditText field(LinearLayout parent, String label, int inputType) {
        EditText edit = new EditText(this);
        edit.setSingleLine(true);
        edit.setHint(label);
        edit.setInputType(inputType);
        parent.addView(edit);
        return edit;
    }

    private TextView addText(LinearLayout parent, String value) {
        TextView text = new TextView(this);
        text.setText(value);
        text.setTextSize(16);
        parent.addView(text);
        return text;
    }

    private int dp(int value) {
        return (int) (value * getResources().getDisplayMetrics().density + 0.5f);
    }

    private interface Work {
        RootClient.Result execute();
        void done(RootClient.Result result);
    }
}
