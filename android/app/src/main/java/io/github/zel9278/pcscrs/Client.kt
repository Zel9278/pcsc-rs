package io.github.zel9278.pcscrs

import android.content.Context

/**
 * Runs the client the same way as scripts/android-adb.sh: copied to /data/local/tmp/pcsc-rs
 * and kept running by a detached loop that restarts it 5 seconds after it exits (crash or
 * self-update). The loop does not depend on this app, so it keeps running when the app is
 * closed, until it is stopped or the phone restarts.
 */
object Client {
    const val DIR = "/data/local/tmp/pcsc-rs"

    /** The client in the APK; extracted because of `useLegacyPackaging` */
    private fun bundled(context: Context) = "${context.applicationInfo.nativeLibraryDir}/libpcsc.so"

    private const val LOOP = """#!/system/bin/sh
cd "${'$'}(dirname "${'$'}0")" || exit 1
echo ${'$'}${'$'} > loop.pid
# The name comes from .env (or the device model), not the shell's HOSTNAME
unset HOSTNAME
# Started from root (through the client's --as-shell-user), TMPDIR may be missing or
# the app's, which the shell user cannot write
export TMPDIR=/data/local/tmp
# Runs while loop.pid is this loop's: stopping removes it, so a loop that could not be
# killed (left by an older version as root) still ends
while [ "${'$'}(cat loop.pid 2>/dev/null)" = "${'$'}${'$'}" ]; do
  ./pcsc-rs
  sleep 5
done
"""

    // Each step may find nothing to stop; `|| true` keeps it from ending a `set -e` script
    private const val STOP =
        "if [ -f $DIR/loop.pid ]; then kill \$(cat $DIR/loop.pid) 2>/dev/null || true; fi; " +
            "rm -f $DIR/loop.pid; pkill -x pcsc-rs 2>/dev/null || true"

    /** Writes `text` to `path` through a quoted here-document, so nothing in it is expanded. */
    private fun write(path: String, text: String): String {
        var end = "PCSC_EOF"
        while (text.contains(end)) end += "_"
        return "cat > $path <<'$end'\n${text.trimEnd('\n')}\n$end"
    }

    private fun env(settings: Settings): String = buildString {
        append("PASS='${settings.pass}'\n")
        // The client exits after updating itself; the loop starts the new version
        append("PCSC_UPDATED=terminate\n")
        if (settings.hostname.isNotBlank()) append("HOSTNAME='${settings.hostname.trim()}'\n")
        if (settings.uri.isNotBlank()) append("PCSC_URI='${settings.uri.trim()}'\n")
    }

    suspend fun start(context: Context, shell: PrivilegedShell, settings: Settings): ShellResult {
        // Copy the APK's client only when the APK changed, so a client that updated itself
        // is not replaced by an older one on every start
        val version = BuildConfig.VERSION_CODE
        val script = listOf(
            "set -e",
            // The here-documents below need a writable TMPDIR, which su does not set
            "export TMPDIR=/data/local/tmp",
            // Shizuku's process has umask 0; only the shell user (and root) may write here
            "umask 022",
            // Left by an older version run through Shizuku started by root
            "if [ \"\$(id -u)\" != 0 ] && [ -d $DIR ] && [ ! -w $DIR ]; then",
            "  echo '$DIR belongs to root (left by an older version). Start once with root or Shizuku started by root to fix it.'",
            "  exit 1",
            "fi",
            "mkdir -p $DIR",
            "chmod 755 $DIR",
            "cd $DIR",
            STOP,
            // One command per line: set -e does not stop at a failure inside an && list
            "if [ ! -x pcsc-rs ] || [ \"\$(cat apk-version 2>/dev/null)\" != $version ]; then",
            "  cp '${bundled(context)}' pcsc-rs.new",
            "  chmod 755 pcsc-rs.new",
            "  mv -f pcsc-rs.new pcsc-rs",
            "  echo $version > apk-version",
            "fi",
            write(".env", env(settings)),
            write("loop.sh", LOOP),
            "chmod 600 .env",
            "chmod 755 pcsc-rs loop.sh",
            // A fresh log, owned by whoever runs the loop
            "rm -f pcsc-rs.log",
            "touch pcsc-rs.log",
            // With root (root mode, or Shizuku started by root), the files are the shell user's
            // and the loop runs as the shell user, with adb's groups: the APK's copy of the
            // client (which the shell user cannot change) switches and starts it. Every way
            // can then update, stop and start the same client.
            "if [ \"\$(id -u)\" = 0 ]; then",
            "  chown -R 2000:2000 $DIR",
            "  rm -f /data/local/tmp/pcsc-rs-ca.pem",
            "  (setsid '${bundled(context)}' --as-shell-user /system/bin/sh ./loop.sh >> pcsc-rs.log 2>&1 < /dev/null &)",
            "else",
            "  (setsid ./loop.sh >> pcsc-rs.log 2>&1 < /dev/null &)",
            "fi",
            "echo started",
        ).joinToString("\n")
        return shell.run(script)
    }

    suspend fun stop(shell: PrivilegedShell): ShellResult = shell.run(STOP)

    data class Status(
        val running: Boolean,
        /** From the log: `Checking current version... v2.5.0` */
        val version: String?,
        val connected: Boolean?,
        val log: String,
    )

    suspend fun status(shell: PrivilegedShell, lines: Int = 40): Status? {
        val result = shell.run(
            "echo \"pids: \$(pidof pcsc-rs)\"; echo ---; tail -n $lines $DIR/pcsc-rs.log 2>/dev/null; true",
        )
        if (!result.ok) return null
        val pids = result.output.lineSequence().first().removePrefix("pids:").trim()
        val log = result.output.substringAfter("---\n", "")
        val logLines = log.lines()
        val version = logLines.lastOrNull { it.startsWith("Checking current version... ") }
            ?.removePrefix("Checking current version... ")?.trim()
        // The last connection event in the log
        val lastEvent = logLines.lastOrNull {
            it.startsWith("Received hi") || it.startsWith("Connection failed") || it.contains("refused") ||
                it.startsWith("Disconnected")
        }
        return Status(
            running = pids.isNotEmpty(),
            version = version,
            connected = lastEvent?.let { it.startsWith("Received hi") },
            log = log,
        )
    }

    /** Settings left by an earlier install (adb or another phone setup), to fill in the first time. */
    suspend fun installedEnv(shell: PrivilegedShell): Map<String, String> {
        val result = shell.run("cat $DIR/.env 2>/dev/null; true")
        return result.output.lineSequence()
            .mapNotNull { line ->
                val key = line.substringBefore('=', "").trim()
                if (key.isEmpty()) return@mapNotNull null
                val value = line.substringAfter('=').trim().removeSurrounding("'").removeSurrounding("\"")
                key to value
            }
            .toMap()
    }
}
