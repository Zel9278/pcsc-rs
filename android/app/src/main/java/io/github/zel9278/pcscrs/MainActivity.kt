package io.github.zel9278.pcscrs

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableIntState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import rikka.shizuku.Shizuku

class MainActivity : ComponentActivity() {
    /** Bumped whenever Shizuku's state may have changed, so the screen reads it again */
    private val shizukuChanges = mutableIntStateOf(0)
    private val onBinder = Shizuku.OnBinderReceivedListener { shizukuChanges.intValue++ }
    private val onBinderDead = Shizuku.OnBinderDeadListener { shizukuChanges.intValue++ }
    private val onPermission = Shizuku.OnRequestPermissionResultListener { _, _ -> shizukuChanges.intValue++ }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        Shizuku.addBinderReceivedListenerSticky(onBinder)
        Shizuku.addBinderDeadListener(onBinderDead)
        Shizuku.addRequestPermissionResultListener(onPermission)
        setContent { AppTheme { App(shizukuChanges) } }
    }

    override fun onDestroy() {
        Shizuku.removeBinderReceivedListener(onBinder)
        Shizuku.removeBinderDeadListener(onBinderDead)
        Shizuku.removeRequestPermissionResultListener(onPermission)
        super.onDestroy()
    }
}

@Composable
private fun AppTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val context = LocalContext.current
    val colors = when {
        Build.VERSION.SDK_INT >= 31 -> if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        dark -> darkColorScheme()
        else -> lightColorScheme()
    }
    MaterialTheme(colorScheme = colors, content = content)
}

private fun shizukuInstalled(context: Context) = try {
    context.packageManager.getPackageInfo("moe.shizuku.privileged.api", 0)
    true
} catch (_: PackageManager.NameNotFoundException) {
    false
}

@Composable
private fun App(shizukuChanges: MutableIntState) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var settings by remember { mutableStateOf(Settings.load(context)) }
    fun update(next: Settings) {
        settings = next
        next.save(context)
    }

    val changes = shizukuChanges.intValue
    val shizuku = remember(changes) { ShizukuShell.state() }
    var rootError by remember { mutableStateOf<String?>(null) }
    var status by remember { mutableStateOf<Client.Status?>(null) }
    var message by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    // The mode chosen before, if it can still be used. Shizuku is picked by itself once allowed
    val mode = when {
        settings.mode == Mode.ROOT -> Mode.ROOT
        shizuku == ShizukuShell.State.READY -> Mode.SHIZUKU
        else -> null
    }
    LaunchedEffect(mode) {
        if (mode == null) return@LaunchedEffect
        if (settings.mode != mode) update(settings.copy(mode = mode))
        // First time: take PASS and the name from a client installed earlier (adb script)
        if (settings.pass.isEmpty()) {
            val env = Client.installedEnv(mode.shell())
            val pass = env["PASS"].orEmpty()
            if (pass.isNotEmpty() && isEnvSafe(pass)) {
                update(
                    settings.copy(
                        pass = pass,
                        hostname = env["HOSTNAME"].orEmpty().takeIf(::isEnvSafe).orEmpty(),
                        uri = env["PCSC_URI"].orEmpty().takeIf(::isEnvSafe).orEmpty(),
                    ),
                )
                message = context.getString(R.string.imported)
            }
        }
    }

    // Check the client every 3 seconds while the screen is shown
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(mode) {
        val shell = mode?.shell() ?: return@LaunchedEffect
        lifecycle.repeatOnLifecycle(Lifecycle.State.RESUMED) {
            while (true) {
                status = Client.status(shell)
                delay(3_000)
            }
        }
    }

    fun act(block: suspend () -> ShellResult) {
        busy = true
        scope.launch {
            val result = block()
            if (!result.ok) message = context.getString(R.string.failed, result.output.ifBlank { "exit ${result.code}" })
            mode?.let { status = Client.status(it.shell()) }
            busy = false
        }
    }

    Scaffold { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(stringResource(R.string.app_name), style = MaterialTheme.typography.headlineMedium)
            Text(stringResource(R.string.subtitle), style = MaterialTheme.typography.bodyMedium)
            message?.let { Text(it, color = MaterialTheme.colorScheme.primary) }

            Section(stringResource(R.string.access_title)) {
                Text(stringResource(R.string.access_explain), style = MaterialTheme.typography.bodySmall)
                ModeOption(
                    selected = mode == Mode.ROOT,
                    label = if (settings.mode == Mode.ROOT) stringResource(R.string.root_ok) else stringResource(R.string.use_root),
                    onSelect = {
                        scope.launch {
                            val result = RootShell.run("id -u")
                            if (result.ok && result.output.trim() == "0") {
                                rootError = null
                                update(settings.copy(mode = Mode.ROOT))
                            } else {
                                rootError = context.getString(R.string.root_failed, result.output.ifBlank { "exit ${result.code}" })
                            }
                        }
                    },
                )
                rootError?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                ModeOption(
                    selected = mode == Mode.SHIZUKU,
                    enabled = shizuku == ShizukuShell.State.READY,
                    label = when (shizuku) {
                        ShizukuShell.State.READY -> stringResource(R.string.shizuku_ready)
                        ShizukuShell.State.NEEDS_PERMISSION -> stringResource(R.string.shizuku_needs_permission)
                        ShizukuShell.State.DENIED -> stringResource(R.string.shizuku_denied)
                        ShizukuShell.State.UNSUPPORTED -> stringResource(R.string.shizuku_unsupported)
                        ShizukuShell.State.NOT_RUNNING -> stringResource(R.string.shizuku_not_running)
                    },
                    onSelect = { update(settings.copy(mode = Mode.SHIZUKU)) },
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (shizuku == ShizukuShell.State.NEEDS_PERMISSION) {
                        Button(onClick = { Shizuku.requestPermission(ShizukuShell.PERMISSION_REQUEST) }) {
                            Text(stringResource(R.string.allow))
                        }
                    }
                    if (shizuku == ShizukuShell.State.NOT_RUNNING && !shizukuInstalled(context)) {
                        TextButton(onClick = {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://shizuku.rikka.app/")))
                        }) { Text(stringResource(R.string.get_shizuku)) }
                    }
                }
                if (mode == null && shizuku != ShizukuShell.State.NEEDS_PERMISSION) {
                    Text(stringResource(R.string.no_access), style = MaterialTheme.typography.bodySmall)
                }
            }

            if (mode != null) {
                Section(stringResource(R.string.status_title)) {
                    val s = status
                    Text(
                        stringResource(if (s?.running == true) R.string.status_running else R.string.status_stopped),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    if (s?.running == true && s.connected != null) {
                        Text(stringResource(if (s.connected) R.string.status_connected else R.string.status_not_connected))
                    }
                    s?.version?.let { Text(stringResource(R.string.status_version, it)) }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        val canStart = settings.pass.isNotEmpty() && listOf(settings.pass, settings.hostname, settings.uri).all(::isEnvSafe)
                        Button(
                            enabled = !busy && canStart,
                            onClick = { act { Client.start(context, mode.shell(), settings, root = mode == Mode.ROOT) } },
                        ) { Text(stringResource(if (s?.running == true) R.string.restart else R.string.start)) }
                        OutlinedButton(enabled = !busy && s?.running == true, onClick = { act { Client.stop(mode.shell()) } }) {
                            Text(stringResource(R.string.stop))
                        }
                    }
                    if (settings.pass.isEmpty()) {
                        Text(stringResource(R.string.pass_needed), color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                    }
                    Text(stringResource(R.string.keeps_running), style = MaterialTheme.typography.bodySmall)
                }
            }

            Section(stringResource(R.string.settings_title)) {
                EnvField(stringResource(R.string.pass), settings.pass, password = true) { update(settings.copy(pass = it)) }
                EnvField(
                    stringResource(R.string.hostname),
                    settings.hostname,
                    placeholder = stringResource(R.string.hostname_hint, Build.MODEL),
                ) { update(settings.copy(hostname = it)) }
                EnvField(
                    stringResource(R.string.uri),
                    settings.uri,
                    placeholder = stringResource(R.string.uri_hint),
                    keyboard = KeyboardType.Uri,
                ) { update(settings.copy(uri = it)) }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(stringResource(R.string.start_on_boot), Modifier.weight(1f))
                    Switch(checked = settings.startOnBoot, onCheckedChange = { update(settings.copy(startOnBoot = it)) })
                }
                if (mode == Mode.SHIZUKU) {
                    Text(stringResource(R.string.start_on_boot_shizuku), style = MaterialTheme.typography.bodySmall)
                }
            }

            if (mode != null) {
                Section(stringResource(R.string.log_title)) {
                    val log = status?.log.orEmpty()
                    Text(
                        log.ifBlank { stringResource(R.string.log_empty) },
                        fontFamily = FontFamily.Monospace,
                        fontSize = 11.sp,
                        lineHeight = 14.sp,
                    )
                }
            }
        }
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            content()
        }
    }
}

@Composable
private fun ModeOption(selected: Boolean, label: String, enabled: Boolean = true, onSelect: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .selectable(selected = selected, enabled = enabled, role = Role.RadioButton, onClick = onSelect),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RadioButton(selected = selected, enabled = enabled, onClick = null)
        Spacer(Modifier.width(8.dp))
        Text(label)
    }
}

@Composable
private fun EnvField(
    label: String,
    value: String,
    placeholder: String? = null,
    password: Boolean = false,
    keyboard: KeyboardType = if (password) KeyboardType.Password else KeyboardType.Text,
    onChange: (String) -> Unit,
) {
    val valid = isEnvSafe(value)
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        placeholder = placeholder?.let { { Text(it) } },
        singleLine = true,
        isError = !valid,
        supportingText = if (valid) null else ({ Text(stringResource(R.string.no_quote)) }),
        visualTransformation = if (password) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
        keyboardOptions = KeyboardOptions(keyboardType = keyboard),
        modifier = Modifier.fillMaxWidth(),
    )
}
