package eu.depau.quoquendo

import android.content.Intent
import android.net.Uri
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
import androidx.compose.material.icons.outlined.GraphicEq
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Language
import androidx.compose.material.icons.outlined.RecordVoiceOver
import androidx.compose.material.icons.outlined.Storage
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
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
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
    summary: String,
    onClick: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    ListItem(
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
        leadingContent = { Icon(icon, contentDescription = null) },
        headlineContent = { Text(title) },
        supportingContent = { Text(summary) },
        trailingContent = trailing,
    )
}

@Composable
fun SettingsScreen(model: MainModel, go: (Screen) -> Unit, modifier: Modifier) {
    val context = LocalContext.current
    var editingMirror by remember { mutableStateOf(false) }
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
                        Text("Set quoquendo as the default engine", style = MaterialTheme.typography.titleMedium)
                    }
                    Spacer(Modifier.height(6.dp))
                    Text(
                        buildAnnotatedString {
                            append("Apps read aloud with Android's default engine. Choose quoquendo under ")
                            withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append("Preferred engine") }
                            append(".")
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(start = 36.dp),
                    )
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

        Section("Engine")
        val ready = model.installed.filter { it.ready }.flatMap { v -> v.locales.map { it.toLanguageTag() } }.distinct()
        val needing = model.installed.filter { it.unsupported == null && it.languageMissing }.flatMap { v -> v.locales.map { it.toLanguageTag() } }.distinct()
        val offered = buildString {
            append(if (ready.isEmpty()) "None yet: install a voice" else ready.joinToString(", "))
            if (needing.isNotEmpty()) append(" · ${needing.joinToString(", ")} needs language data")
        }
        Setting(Icons.Outlined.Language, "Languages offered to apps", offered)
        Setting(
            Icons.Outlined.GraphicEq,
            "Show how speech is built",
            "On the Try it screen",
            onClick = { model.updateShowCurtain(!model.showCurtain) },
        ) { Switch(checked = model.showCurtain, onCheckedChange = model::updateShowCurtain) }

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
            "Only while quoquendo is open",
            onClick = { model.updateWifiOnly(!model.wifiOnly) },
        ) { Switch(checked = model.wifiOnly, onCheckedChange = model::updateWifiOnly) }

        Section("About")
        Setting(
            Icons.Outlined.Info,
            "quoquendo ${BuildConfig.VERSION_NAME}",
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
            text = {
                Column {
                    Text("Voices download from this mirror first, and from archive.org when it lacks one. Leave it empty to use archive.org only.")
                    Spacer(Modifier.height(12.dp))
                    OutlinedTextField(value = url, onValueChange = { url = it }, singleLine = true, label = { Text("URL") }, modifier = Modifier.fillMaxWidth())
                }
            },
            dismissButton = { TextButton(onClick = { editingMirror = false }) { Text("Cancel") } },
            confirmButton = { TextButton(onClick = { model.updateMirror(url); editingMirror = false }) { Text("Save") } },
        )
    }
}
