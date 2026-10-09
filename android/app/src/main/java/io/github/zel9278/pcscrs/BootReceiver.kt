package io.github.zel9278.pcscrs

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/**
 * Starts the client after a restart when "start on boot" is on. With Shizuku this works
 * only when Shizuku itself starts on boot (Sui, or Shizuku started by root); otherwise
 * open the app and start it once Shizuku is running.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val settings = Settings.load(context)
        val mode = settings.mode ?: return
        if (!settings.startOnBoot || settings.pass.isEmpty()) return

        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                // A boot broadcast may take a little while; wait at most 25 seconds for Shizuku
                val ready = mode == Mode.ROOT || withTimeoutOrNull(25_000) {
                    while (ShizukuShell.state() != ShizukuShell.State.READY) delay(1_000)
                    true
                } == true
                if (ready) Client.start(context, mode.shell(), settings, root = mode == Mode.ROOT)
            } finally {
                pending.finish()
            }
        }
    }
}
