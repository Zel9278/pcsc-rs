package io.github.zel9278.pcscrs

import android.content.ComponentName
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.IBinder
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import rikka.shizuku.Shizuku
import java.io.BufferedReader
import java.io.IOException
import java.io.Writer
import java.util.UUID
import kotlin.coroutines.resume

data class ShellResult(val code: Int, val output: String) {
    val ok get() = code == 0
}

/** How the app gets the rights to run the client. */
enum class Mode { ROOT, SHIZUKU }

interface PrivilegedShell {
    suspend fun run(command: String): ShellResult
}

/**
 * One `su` process kept open, so the root manager asks (and shows its toast) once
 * instead of on every status check.
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
        val (stdin, stdout) = open()
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

    private fun open(): Pair<Writer, BufferedReader> {
        val running = process?.takeIf { it.alive() }
        if (running != null) return input!! to output!!
        val started = ProcessBuilder("su").redirectErrorStream(true).start()
        process = started
        input = started.outputStream.bufferedWriter()
        output = started.inputStream.bufferedReader()
        return input!! to output!!
    }

    private fun close() {
        process?.destroy()
        process = null
    }
}

/** Runs commands through [ShellService] in the process Shizuku starts for this app. */
object ShizukuShell : PrivilegedShell {
    const val PERMISSION_REQUEST = 1

    private val lock = Mutex()
    private var service: IShell? = null

    enum class State { NOT_RUNNING, UNSUPPORTED, NEEDS_PERMISSION, DENIED, READY }

    fun state(): State = when {
        !Shizuku.pingBinder() -> State.NOT_RUNNING
        // Shizuku before v11 has no user services
        Shizuku.isPreV11() -> State.UNSUPPORTED
        Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED -> State.READY
        Shizuku.shouldShowRequestPermissionRationale() -> State.DENIED
        else -> State.NEEDS_PERMISSION
    }

    override suspend fun run(command: String): ShellResult {
        val shell = lock.withLock { service?.takeIf { it.asBinder().pingBinder() } ?: bind().also { service = it } }
        return withContext(Dispatchers.IO) {
            val raw = shell.exec(command)
            val code = raw.substringBefore('\n').toIntOrNull() ?: -1
            ShellResult(code, raw.substringAfter('\n').trimEnd('\n'))
        }
    }

    private suspend fun bind(): IShell = withTimeout(15_000) {
        suspendCancellableCoroutine { continuation ->
            val args = Shizuku.UserServiceArgs(
                ComponentName(BuildConfig.APPLICATION_ID, ShellService::class.java.name),
            )
                .processNameSuffix("shell")
                .debuggable(BuildConfig.DEBUG)
                .version(BuildConfig.VERSION_CODE)
            val connection = object : ServiceConnection {
                override fun onServiceConnected(name: ComponentName?, binder: IBinder?) {
                    if (binder != null && binder.pingBinder() && continuation.isActive) {
                        continuation.resume(IShell.Stub.asInterface(binder))
                    }
                }

                override fun onServiceDisconnected(name: ComponentName?) {
                    service = null
                }
            }
            Shizuku.bindUserService(args, connection)
        }
    }
}

fun Mode.shell(): PrivilegedShell = when (this) {
    Mode.ROOT -> RootShell
    Mode.SHIZUKU -> ShizukuShell
}

// Process.isAlive needs API 26
private fun Process.alive(): Boolean = try {
    exitValue()
    false
} catch (_: IllegalThreadStateException) {
    true
}
