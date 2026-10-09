package io.github.zel9278.pcscrs

import android.content.Context
import androidx.core.content.edit

/** What the user set in the app. PASS is kept in the app's private storage only. */
data class Settings(
    val pass: String = "",
    val hostname: String = "",
    /** Empty: the client's default server */
    val uri: String = "",
    val mode: Mode? = null,
    val startOnBoot: Boolean = false,
) {
    companion object {
        private const val FILE = "settings"

        fun load(context: Context): Settings {
            val p = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
            return Settings(
                pass = p.getString("pass", "").orEmpty(),
                hostname = p.getString("hostname", "").orEmpty(),
                uri = p.getString("uri", "").orEmpty(),
                mode = p.getString("mode", null)?.let { name -> Mode.entries.find { it.name == name } },
                startOnBoot = p.getBoolean("startOnBoot", false),
            )
        }
    }

    fun save(context: Context) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit {
            putString("pass", pass)
            putString("hostname", hostname)
            putString("uri", uri)
            putString("mode", mode?.name)
            putBoolean("startOnBoot", startOnBoot)
        }
    }
}

/** .env takes values in single quotes literally; they cannot hold a quote or a line break. */
fun isEnvSafe(value: String) = value.none { it == '\'' || it == '\n' || it == '\r' }
