package io.github.zel9278.pcscrs

import android.content.Context
import kotlin.system.exitProcess

/** Started by Shizuku as the shell user (the same rights as `adb shell`). */
class ShellService() : IShell.Stub() {
    @Suppress("unused") // Shizuku passes a context when the constructor takes one
    constructor(context: Context) : this()

    override fun destroy() {
        exitProcess(0)
    }

    override fun exec(command: String): String {
        val process = ProcessBuilder("sh", "-c", command).redirectErrorStream(true).start()
        val output = process.inputStream.bufferedReader().readText()
        return "${process.waitFor()}\n$output"
    }
}
