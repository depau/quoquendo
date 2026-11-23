package eu.depau.quoquendo

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.CloudDownload
import androidx.compose.material.icons.outlined.FileDownload
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.RecordVoiceOver
import androidx.compose.material.icons.outlined.Storage
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsTopBar() {
    TopAppBar(
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
        title = { Text("Settings") },
    )
}

@Composable
private fun Section(title: String) {
    Text(
        title,
        color = MaterialTheme.colorScheme.primary,
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 4.dp),
    )
}

@Composable
private fun Setting(
    icon: ImageVector,
    title: String,
    summary: String?,
    onClick: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    ListItem(
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
        leadingContent = { Icon(icon, contentDescription = null) },
        headlineContent = { Text(title) },
        supportingContent = summary?.let { { Text(it) } },
        trailingContent = trailing,
    )
}

private const val MATCH_THE_REGION = "Match the region"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(model: MainModel, go: (Screen) -> Unit, modifier: Modifier) {
    val context = LocalContext.current
    var editingMirror by remember { mutableStateOf(false) }
    var choosingFor by remember { mutableStateOf<String?>(null) }
    val backup = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { uri ->
        if (uri != null) {
            runCatching {
                context.contentResolver.openOutputStream(uri)?.use { it.write(model.backupSettings().toByteArray()) }
            }.onSuccess { model.message("Settings backed up") }.onFailure { model.message("Could not back up: ${it.message}") }
        }
    }
    val restore = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) {
            val json = runCatching { context.contentResolver.openInputStream(uri)?.bufferedReader()?.readText().orEmpty() }.getOrDefault("")
            val error = model.restoreSettings(json)
            model.message(error ?: "Settings restored")
        }
    }
    Column(modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
        if (!model.isDefaultEngine) {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                Column(Modifier.padding(16.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Outlined.RecordVoiceOver, contentDescription = null)
                        Spacer(Modifier.width(12.dp))
                        Text("Set Quoquendo as the default engine", style = MaterialTheme.typography.titleMedium)
                    }
                    Spacer(Modifier.height(12.dp))
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        Button(
                            onClick = { openTtsSettings(context) },
                            colors = ButtonDefaults.buttonColors(
                                containerColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                contentColor = MaterialTheme.colorScheme.onPrimary,
                            ),
                        ) {
                            Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, Modifier.size(18.dp))
                            Spacer(Modifier.width(8.dp))
                            Text("Open system TTS settings")
                        }
                    }
                }
            }
        }

        val languages = model.appLanguages()
        if (languages.isNotEmpty()) {
            Section("Voice apps use")
            for (language in languages) {
                Setting(
                    Icons.Outlined.RecordVoiceOver,
                    languageName(language),
                    model.appVoice(language)?.name ?: MATCH_THE_REGION,
                    onClick = { choosingFor = language },
                ) { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) }
            }
        }

        Section("Text")
        Setting(
            Icons.Outlined.Tune,
            "Profile",
            model.activeProfile,
            onClick = { go(Screen.Profiles) },
        ) { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) }

        Section("Engine")
        Setting(
            Icons.Outlined.Tune,
            "Advanced",
            null,
            onClick = { model.updateAdvanced(!model.advanced) },
        ) { Switch(checked = model.advanced, onCheckedChange = model::updateAdvanced) }

        Section("Voices")
        Setting(
            Icons.Outlined.Storage,
            "Storage",
            "${size(model.bytesUsed)} in ${model.installed.size} voice${if (model.installed.size == 1) "" else "s"}",
            onClick = { go(Screen.Voices) },
        ) { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) }
        Setting(
            Icons.Outlined.CloudDownload,
            "Voice mirror",
            model.mirror.ifEmpty { "None: voices come from archive.org" },
            onClick = { editingMirror = true },
        ) { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) }
        Setting(
            Icons.Outlined.FileDownload,
            "Download on Wi-Fi only",
            "Only while Quoquendo is open",
            onClick = { model.updateWifiOnly(!model.wifiOnly) },
        ) { Switch(checked = model.wifiOnly, onCheckedChange = model::updateWifiOnly) }

        Section("Backup")
        Setting(
            Icons.Outlined.FileDownload,
            "Back up settings",
            "Profiles and speech settings, as JSON",
            onClick = { backup.launch("quoquendo-settings.json") },
        )
        Setting(
            Icons.Outlined.FileDownload,
            "Restore from file",
            "Replace profiles and speech settings",
            onClick = { restore.launch(arrayOf("application/json")) },
        )

        Section("About")
        Setting(
            Icons.Outlined.Info,
            "Quoquendo ${BuildConfig.VERSION_NAME}",
            "Open-source Loquendo 7 rebuild",
            onClick = { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/depau/quoquendo"))) },
        ) { Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null) }
        Spacer(Modifier.height(16.dp))
    }

    if (editingMirror) {
        var url by remember { mutableStateOf(model.mirror) }
        AlertDialog(
            onDismissRequest = { editingMirror = false },
            title = { Text("Voice mirror") },
            text = { OutlinedTextField(value = url, onValueChange = { url = it }, singleLine = true, label = { Text("URL") }, modifier = Modifier.fillMaxWidth()) },
            dismissButton = { TextButton(onClick = { editingMirror = false }) { Text("Cancel") } },
            confirmButton = { TextButton(onClick = { model.updateMirror(url); editingMirror = false }) { Text("Save") } },
        )
    }

    choosingFor?.let { language -> VoiceChooser(model, language) { choosingFor = null } }
}

/** "Voice for <Language>": Match the region (when its voices come from several), or one voice for all of it. */
@Composable
private fun VoiceChooser(model: MainModel, language: String, onDismiss: () -> Unit) {
    val chosen = model.appVoice(language)?.name
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Voice for ${languageName(language)}") },
        text = {
            Column {
                @Composable
                fun Option(title: String, detail: String?, selected: Boolean, flag: String? = null, onClick: () -> Unit) {
                    Row(
                        Modifier.fillMaxWidth().clickable { onClick(); onDismiss() }.padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = selected, onClick = null)
                        Spacer(Modifier.width(16.dp))
                        Column {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                if (flag != null) Flag(flag, Modifier.padding(end = 8.dp).size(24.dp, 17.dp))
                                Text(title, style = MaterialTheme.typography.bodyLarge)
                            }
                            if (detail != null) {
                                Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }
                if (model.regions(language) > 1) Option(MATCH_THE_REGION, null, chosen == null) { model.chooseAppVoice(language, null) }
                for (v in model.speakersOf(language)) {
                    val region = v.locales.firstOrNull { it.language == language }?.displayCountry.orEmpty()
                    Option(v.name, listOf(region, v.gender).filter { it.isNotEmpty() }.joinToString(" · "), v.name == chosen, flagLanguage(v.name, model.languageOf(v))) {
                        model.chooseAppVoice(language, v.name)
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
