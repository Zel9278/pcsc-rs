package io.github.zel9278.pcscrs

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.ActivityNotFoundException
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.provider.Settings as AndroidSettings
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.RemoteInput
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * Pairing with wireless debugging. The pairing code is shown in a dialog in Settings that
 * closes when the user leaves it, so the code is typed into a notification instead.
 */
object Pairing {
    private const val CHANNEL = "pairing"
    private const val NOTIFICATION = 1
    const val KEY_CODE = "code"

    private fun channel(context: Context) {
        if (Build.VERSION.SDK_INT < 26) return
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel(CHANNEL, context.getString(R.string.pairing_channel), NotificationManager.IMPORTANCE_HIGH),
        )
    }

    /** Asks for the code; [error] is shown above the input when the last try failed. */
    fun ask(context: Context, error: String? = null) {
        channel(context)
        val reply = PendingIntent.getBroadcast(
            context,
            0,
            Intent(context, PairingReceiver::class.java),
            // RemoteInput writes the code into the intent, so it must be mutable
            PendingIntent.FLAG_UPDATE_CURRENT or (if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0),
        )
        val input = RemoteInput.Builder(KEY_CODE).setLabel(context.getString(R.string.pairing_code)).build()
        val action = NotificationCompat.Action.Builder(0, context.getString(R.string.pairing_enter), reply)
            .addRemoteInput(input)
            .build()
        post(
            context,
            NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle(context.getString(R.string.pairing_title))
                .setContentText(error ?: context.getString(R.string.pairing_text))
                .setStyle(NotificationCompat.BigTextStyle().bigText(error ?: context.getString(R.string.pairing_text)))
                .setPriority(NotificationCompat.PRIORITY_HIGH)
                .setOngoing(true)
                .addAction(action),
        )
    }

    fun done(context: Context) {
        post(
            context,
            NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle(context.getString(R.string.pairing_done))
                .setContentText(context.getString(R.string.pairing_done_text))
                .setAutoCancel(true)
                .setTimeoutAfter(15_000),
        )
    }

    @Suppress("MissingPermission") // asked for before pairing starts; without it nothing is shown
    private fun post(context: Context, builder: NotificationCompat.Builder) {
        runCatching { NotificationManagerCompat.from(context).notify(NOTIFICATION, builder.build()) }
    }

    fun cancel(context: Context) = NotificationManagerCompat.from(context).cancel(NOTIFICATION)

    /** Developer options, scrolled to wireless debugging */
    fun openSettings(context: Context) {
        val intent = Intent(AndroidSettings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS)
            .putExtra(":settings:fragment_args_key", "toggle_adb_wireless")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            context.startActivity(Intent(AndroidSettings.ACTION_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }
}

/** Receives the code typed into the notification and pairs. */
class PairingReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val code = RemoteInput.getResultsFromIntent(intent)?.getCharSequence(Pairing.KEY_CODE)?.toString()?.trim()
        if (code.isNullOrEmpty()) return
        AdbShell.init(context)
        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                val result = AdbShell.pair(code)
                if (result.isSuccess) {
                    Settings.load(context).copy(mode = Mode.ADB, paired = true).save(context)
                    // While connected, let the app switch wireless debugging on by itself later
                    if (AdbShell.connect(10_000) == AdbShell.State.READY) AdbShell.grantSwitch(context)
                    Pairing.done(context)
                } else {
                    val reason = result.exceptionOrNull()?.message ?: "?"
                    Pairing.ask(context, context.getString(R.string.pairing_failed, reason))
                }
            } finally {
                pending.finish()
            }
        }
    }
}
