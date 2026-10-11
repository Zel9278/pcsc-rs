package io.github.zel9278.pcscrs

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.IOException
import java.io.Writer
import java.util.UUID

data class ShellResult(val code: Int, val output: String) {
    val ok get() = code == 0
}

/** How the app gets the rights to run the client. */
enum class Mode {
    ROOT,

    /** This phone's own adbd over wireless debugging: the shell user, no root needed */
    ADB,

    /** Shizuku's process (the shell user when Shizuku was started with adb) */
    SHIZUKU,
}

interface PrivilegedShell {
    suspend fun run(command: String): ShellResult
}

/**
 * One `su` process kept open, so the root manager asks (and shows its toast) once
 * instead of on every status check.
 */
object RootShell : PrivilegedShell {
    /**
     * The groups adbd gives the shell user. Plain `su 2000` has none of them, and without
     * inet (3003) the process cannot reach netd's DNS socket (`dnsproxyd`, root:inet 0660):
     * every name lookup fails although sockets to addresses work.
     */
    private val SHELL_GROUPS = listOf(
        3003, // inet
        1004, 1007, 1011, 1015, 1028, 1078, 1079, // input, log, adb, sdcard_rw, sdcard_r, ext_data_rw, ext_obb_rw
        3001, 3002, 3006, 3009, 3011, 3012, // net_bt_admin, net_bt, net_bw_stats, readproc, uhid, readtracefs
    )

    private val lock = Mutex()
    private var process: Process? = null
    private var input: Writer? = null
    private var output: BufferedReader? = null

    /** Cleared when this root manager's su does not take -g / -G; then the shell is root */
    private var withGroups = true

    override suspend fun run(command: String): ShellResult = withContext(Dispatchers.IO) {
        lock.withLock {
            try {
                exchange(command)
            } catch (e: IOException) {
                // su missing, denied, or the shell went away
                close()
                ShellResult(-1, e.message.orEmpty())
            }
        }
    }

    private fun exchange(command: String): ShellResult {
        open()
        return send(command)
    }

    private fun send(command: String): ShellResult {
        val stdin = input!!
        val stdout = output!!
        val marker = "__PCSC_END_${UUID.randomUUID()}__"
        // In a subshell, so `exit` or `cd` in the command do not end or move the shell
        stdin.write("(\n$command\n) 2>&1 </dev/null\n__pcsc_rc=\$?\necho\necho \"$marker \$__pcsc_rc\"\n")
        stdin.flush()
        val lines = StringBuilder()
        while (true) {
            val line = stdout.readLine()
            if (line == null) {
                close()
                return ShellResult(-1, lines.toString().trimEnd('\n'))
            }
            if (line.startsWith(marker)) {
                val code = line.removePrefix(marker).trim().toIntOrNull() ?: -1
                return ShellResult(code, lines.toString().trimEnd('\n'))
            }
            lines.append(line).append('\n')
        }
    }

    /**
     * As the shell user (uid 2000) with its groups, like adb and Shizuku: the client never runs
     * as root, its files stay the shell user's (nothing a shell process can plant is run as
     * root), and switching to wireless debugging or Shizuku later can still stop it.
     * Without -g / -G, as root (see below).
     */
    private fun open() {
        if (process?.alive() == true) return
        if (withGroups) {
            // Options before the user: su stops reading options there (Magisk, KernelSU)
            start(listOf("su", "-g", "2000") + SHELL_GROUPS.flatMap { listOf("-G", "$it") } + "2000")
            val id = try {
                send("id -u")
            } catch (_: IOException) {
                null
            }
            if (id?.ok == true && id.output.trim() == "2000") return
            // A su that rejects -g / -G: plain su 2000 would have no groups and could not
            // look up names, so run as root; Client.start then gives the files to the shell
            // user, and the client switches to it with its groups by itself
            close()
            withGroups = false
        }
        start(listOf("su"))
    }

    private fun start(command: List<String>) {
        val started = ProcessBuilder(command).redirectErrorStream(true).start()
        process = started
        input = started.outputStream.bufferedWriter()
        output = started.inputStream.bufferedReader()
    }

    private fun close() {
        process?.destroy()
        process = null
    }
}

fun Mode.shell(): PrivilegedShell = when (this) {
    Mode.ROOT -> RootShell
    Mode.ADB -> AdbShell
    Mode.SHIZUKU -> ShizukuShell
}

// Process.isAlive needs API 26
private fun Process.alive(): Boolean = try {
    exitValue()
    false
} catch (_: IllegalThreadStateException) {
    true
}
