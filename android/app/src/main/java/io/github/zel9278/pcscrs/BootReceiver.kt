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
 * was off and USB debugging is on. The phone has to be on a Wi-Fi network where wireless debugging was allowed before.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val settings = Settings.load(context)
        val mode = settings.mode ?: return
        if (!settings.startOnBoot || settings.pass.isEmpty()) return
        if (!listOf(settings.pass, settings.hostname, settings.uri).all(::isEnvSafe)) return
        AdbShell.init(context)

        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                when (mode) {
                    Mode.ROOT -> Client.start(context, RootShell, settings)
                    Mode.ADB -> startOverAdb(context, settings)
                    Mode.SHIZUKU -> {
                        // Only when Shizuku starts on boot too (Sui, or started by root)
                        val ready = withTimeoutOrNull(25_000) {
                            while (ShizukuShell.state() != ShizukuShell.State.READY) delay(1_000)
                            true
                        } == true
                        if (ready) Client.start(context, ShizukuShell, settings)
                    }
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
        if (ready) Client.start(context, AdbShell, settings)
        // Switching it back off stops adbd when USB debugging is off too, and init then kills
        // everything started through it, the client included. Only switch it off when USB
        // debugging keeps adbd running.
        if (!wasOn && AdbShell.usbDebuggingOn(context)) {
            runCatching { AndroidSettings.Global.putInt(context.contentResolver, "adb_wifi_enabled", 0) }
        }
    }
}
