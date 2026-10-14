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
    /** Paired with wireless debugging at least once */
    val paired: Boolean = false,
    /** Items kept off PC Status: the client's `PCSC_HIDE` names ([HIDE_ITEMS]) */
    val hidden: Set<String> = emptySet(),
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
                paired = p.getBoolean("paired", false),
                hidden = p.getStringSet("hidden", null).orEmpty().filter { it in HIDE_ITEMS }.toSet(),
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
            putBoolean("paired", paired)
            putStringSet("hidden", hidden)
        }
    }
}

/** What the client can keep off PC Status (`PCSC_HIDE`), in the order the app lists them. */
val HIDE_ITEMS = listOf(
    "os", "cpu", "cpu_name", "cpu_frequency", "memory", "storage",
    "gpu", "gpu_name", "gpu_usage", "gpu_memory",
    "temperature", "battery", "uptime", "loadavg", "iowait",
)

/** What `gpu` covers. */
val GPU_PARTS = setOf("gpu_name", "gpu_usage", "gpu_memory")

/** `PCSC_HIDE` from a set of names: comma separated, in list order. */
fun pcscHide(hidden: Set<String>) = HIDE_ITEMS.filter { it in hidden }.joinToString(",")

/** .env takes values in single quotes literally; they cannot hold a quote or a line break. */
fun isEnvSafe(value: String) = value.none { it == '\'' || it == '\n' || it == '\r' }
