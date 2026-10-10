package io.github.zel9278.pcscrs.legacy;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

public final class BootReceiver extends BroadcastReceiver {
    public void onReceive(final Context context, Intent intent) {
        if (!Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) return;
        final Settings settings = Settings.load(context);
        if (!settings.startOnBoot || settings.pass.length() == 0 || !settings.valid()) return;
        new Thread(new Runnable() {
            public void run() { RootClient.start(context, settings); }
        }).start();
    }
}
