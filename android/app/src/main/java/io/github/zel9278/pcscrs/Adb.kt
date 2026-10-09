package io.github.zel9278.pcscrs

import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings as AndroidSettings
import io.github.muntashirakon.adb.AbsAdbConnectionManager
import io.github.muntashirakon.adb.AdbPairingRequiredException
import io.github.muntashirakon.adb.android.AdbMdns
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.bouncycastle.asn1.x500.X500Name
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder
import java.io.File
import java.math.BigInteger
import java.net.InetAddress
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.PrivateKey
import java.security.cert.Certificate
import java.security.cert.CertificateFactory
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Date
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/**
 * This phone's own adbd, reached over wireless debugging (Android 11 and later) or ADB over
 * TCP (`adb tcpip 5555`, older versions). Commands run as the shell user, like `adb shell`.
 *
 * The ADB key is made once and kept in the app's private storage. Pairing it once (with the
 * code from "Pair device with pairing code") makes adbd trust it from then on.
 */
class AdbManager private constructor(private val context: Context) : AbsAdbConnectionManager() {
    private val dir = File(context.filesDir, "adb").apply { mkdirs() }
    private val keyFile = File(dir, "key.pk8")
    private val certFile = File(dir, "cert.der")
    private val privateKey: PrivateKey
    private val certificate: Certificate

    init {
        setApi(Build.VERSION.SDK_INT)
        setTimeout(10, TimeUnit.SECONDS)
        if (!keyFile.exists() || !certFile.exists()) generate()
        privateKey = KeyFactory.getInstance("RSA").generatePrivate(PKCS8EncodedKeySpec(keyFile.readBytes()))
        certificate = certFile.inputStream().use { CertificateFactory.getInstance("X.509").generateCertificate(it) }
    }

    private fun generate() {
        val pair = KeyPairGenerator.getInstance("RSA").apply { initialize(2048) }.generateKeyPair()
        val name = X500Name("CN=pcsc-rs")
        val now = System.currentTimeMillis()
        val holder = JcaX509v3CertificateBuilder(
            name,
            BigInteger.valueOf(now),
            Date(now - 86_400_000L),
            // adbd does not check the dates; keep the key usable for good
            Date(now + 100L * 365 * 86_400_000L),
            name,
            pair.public,
        ).build(JcaContentSignerBuilder("SHA256withRSA").build(pair.private))
        val cert = JcaX509CertificateConverter().getCertificate(holder)
        keyFile.writeBytes(pair.private.encoded)
        certFile.writeBytes(cert.encoded)
    }

    override fun getPrivateKey() = privateKey
    override fun getCertificate() = certificate
    override fun getDeviceName() = "pcsc-rs"

    companion object {
        @SuppressLint("StaticFieldLeak") // the application context
        @Volatile
        private var instance: AdbManager? = null

        fun get(context: Context): AdbManager =
            instance ?: synchronized(this) {
                instance ?: AdbManager(context.applicationContext).also { instance = it }
            }
    }
}

object AdbShell : PrivilegedShell {
    private const val SETTING = "adb_wifi_enabled"

    private val lock = Mutex()
    private lateinit var appContext: Context

    fun init(context: Context) {
        appContext = context.applicationContext
    }

    enum class State {
        /** Connected, commands can run */
        READY,

        /** adbd does not know this app's key yet: pair it */
        NEEDS_PAIRING,

        /** Wireless debugging is off, or the phone is not on Wi-Fi */
        OFF,
    }

    /** Android 11 has wireless debugging; older versions need `adb tcpip 5555` from a PC */
    val hasWirelessDebugging get() = Build.VERSION.SDK_INT >= 30

    fun wirelessDebuggingOn(context: Context): Boolean =
        AndroidSettings.Global.getInt(context.contentResolver, SETTING, 0) == 1

    /** WRITE_SECURE_SETTINGS lets the app switch wireless debugging on by itself; granted through adb */
    fun canSwitch(context: Context): Boolean =
        context.checkSelfPermission(android.Manifest.permission.WRITE_SECURE_SETTINGS) == PackageManager.PERMISSION_GRANTED

    /** Turns wireless debugging on (it may still ask once per Wi-Fi network). False without the permission. */
    fun switchOn(context: Context): Boolean = try {
        canSwitch(context) && AndroidSettings.Global.putInt(context.contentResolver, SETTING, 1)
    } catch (_: SecurityException) {
        false
    }

    /** Connects if not connected yet. */
    suspend fun connect(timeoutMillis: Long = 5_000): State = withContext(Dispatchers.IO) {
        lock.withLock { connectLocked(timeoutMillis) }
    }

    private fun connectLocked(timeoutMillis: Long): State {
        val manager = AdbManager.get(appContext)
        if (manager.isConnected) return State.READY
        return try {
            val connected = if (hasWirelessDebugging) {
                manager.autoConnect(appContext, timeoutMillis)
            } else {
                manager.connect("127.0.0.1", 5555)
            }
            if (connected || manager.isConnected) State.READY else State.OFF
        } catch (_: AdbPairingRequiredException) {
            State.NEEDS_PAIRING
        } catch (_: Exception) {
            // Not found on the network (off), refused, or timed out
            State.OFF
        }
    }

    override suspend fun run(command: String): ShellResult = withContext(Dispatchers.IO) {
        lock.withLock {
            if (connectLocked(5_000) != State.READY) return@withLock ShellResult(-1, "wireless debugging is not connected")
            try {
                exec(command)
            } catch (e: Exception) {
                // The connection went away (wireless debugging switched off, Wi-Fi changed); next time reconnects
                runCatching { AdbManager.get(appContext).disconnect() }
                ShellResult(-1, e.message.orEmpty())
            }
        }
    }

    /** `exec:` gives a plain pipe to `sh` (no terminal), like ProcessBuilder */
    private fun exec(command: String): ShellResult {
        val marker = "__PCSC_END_${UUID.randomUUID()}__"
        AdbManager.get(appContext).openStream("exec:sh").use { stream ->
            stream.openOutputStream().apply {
                write("(\n$command\n) 2>&1 </dev/null\n__pcsc_rc=\$?\necho\necho \"$marker \$__pcsc_rc\"\nexit\n".toByteArray())
                flush()
            }
            val output = stream.openInputStream().readBytes().toString(Charsets.UTF_8)
            val code = output.substringAfterLast(marker, "").trim().toIntOrNull() ?: -1
            return ShellResult(code, output.substringBeforeLast(marker).trimEnd('\n'))
        }
    }

    /**
     * Pairs with the code shown in "Pair device with pairing code". The pairing port is found
     * through mDNS while that dialog is open.
     */
    suspend fun pair(code: String): Result<Unit> = withContext(Dispatchers.IO) {
        lock.withLock {
            runCatching {
                val (host, port) = findPairingService(10_000)
                    ?: error("pairing service not found; keep the pairing code dialog open")
                check(AdbManager.get(appContext).pair(host, port, code.trim())) { "pairing failed" }
            }
        }
    }

    private fun findPairingService(timeoutMillis: Long): Pair<String, Int>? {
        val found = CountDownLatch(1)
        var host: InetAddress? = null
        val port = AtomicInteger(-1)
        val mdns = AdbMdns(appContext, AdbMdns.SERVICE_TYPE_TLS_PAIRING) { address, p ->
            if (address != null) {
                host = address
                port.set(p)
                found.countDown()
            }
        }
        mdns.start()
        try {
            found.await(timeoutMillis, TimeUnit.MILLISECONDS)
        } finally {
            mdns.stop()
        }
        val address = host ?: return null
        return address.hostAddress!! to port.get()
    }

    /** Lets the app switch wireless debugging on by itself (for "start after restart" and the Start button). */
    suspend fun grantSwitch(context: Context): Boolean {
        if (canSwitch(context)) return true
        run("pm grant ${context.packageName} android.permission.WRITE_SECURE_SETTINGS")
        return canSwitch(context)
    }
}
