package io.github.zel9278.pcscrs

import android.content.ComponentName
import android.content.Context
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
import kotlin.coroutines.resume

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


fun shizukuInstalled(context: Context) = try {
    context.packageManager.getPackageInfo("moe.shizuku.privileged.api", 0)
    true
} catch (_: PackageManager.NameNotFoundException) {
    false
}
