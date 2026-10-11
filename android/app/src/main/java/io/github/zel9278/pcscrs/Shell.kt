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
 * instead of on every status check. The shell is root; Client.start gives the files to the
 * shell user and starts the loop as the shell user, so the client itself never runs as root.
 */
object RootShell : PrivilegedShell {
    private val lock = Mutex()
    private var process: Process? = null
    private var input: Writer? = null
    private var output: BufferedReader? = null

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

    private fun open() {
        if (process?.alive() != true) start(listOf("su"))
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
