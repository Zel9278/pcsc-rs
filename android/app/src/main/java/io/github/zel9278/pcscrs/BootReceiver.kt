package io.github.zel9278.pcscrs

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.provider.Settings as AndroidSettings
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/**
 * Starts the client after a restart when "start on boot" is on. With wireless debugging the
 * app switches it on (it can once paired), starts the client, and switches it back off if it
 * was off. The phone has to be on a Wi-Fi network where wireless debugging was allowed before.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val settings = Settings.load(context)
        val mode = settings.mode ?: return
        if (!settings.startOnBoot || settings.pass.isEmpty()) return
        AdbShell.init(context)

        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                when (mode) {
                    Mode.ROOT -> Client.start(context, RootShell, settings, root = true)
                    Mode.ADB -> startOverAdb(context, settings)
                }
            } finally {
                pending.finish()
            }
        }
    }

    private suspend fun startOverAdb(context: Context, settings: Settings) {
        val wasOn = AdbShell.wirelessDebuggingOn(context)
        if (!wasOn && !AdbShell.switchOn(context)) return
        // Wi-Fi may come up a little after boot; a background broadcast has about a minute
        val ready = withTimeoutOrNull(45_000) {
            while (AdbShell.connect(3_000) != AdbShell.State.READY) delay(2_000)
            true
        } == true
        if (ready) Client.start(context, AdbShell, settings, root = false)
        if (!wasOn) {
            runCatching { AndroidSettings.Global.putInt(context.contentResolver, "adb_wifi_enabled", 0) }
        }
    }
}
