package io.github.zel9278.pcscrs.legacy;

import android.content.Context;
import android.content.SharedPreferences;

final class Settings {
    final String pass;
    final String hostname;
    final String uri;
    final boolean startOnBoot;

    Settings(String pass, String hostname, String uri, boolean startOnBoot) {
        this.pass = pass;
        this.hostname = hostname;
        this.uri = uri;
        this.startOnBoot = startOnBoot;
    }

    static Settings load(Context context) {
        SharedPreferences prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE);
        return new Settings(prefs.getString("pass", ""), prefs.getString("hostname", ""),
                prefs.getString("uri", ""), prefs.getBoolean("startOnBoot", false));
    }

    void save(Context context) {
        context.getSharedPreferences("settings", Context.MODE_PRIVATE).edit()
                .putString("pass", pass).putString("hostname", hostname).putString("uri", uri)
                .putBoolean("startOnBoot", startOnBoot).commit();
    }

    boolean valid() {
        return safe(pass) && safe(hostname) && safe(uri);
    }

    private static boolean safe(String value) {
        return value.indexOf('\'') < 0 && value.indexOf('\n') < 0 && value.indexOf('\r') < 0;
    }
}
