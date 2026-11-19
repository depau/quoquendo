package eu.depau.quoquendo

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.CloudDownload
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.FileDownload
import androidx.compose.material.icons.outlined.Storage
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VoicesTopBar(model: MainModel, search: MutableState<String?>) {
    val context = LocalContext.current
    var menu by remember { mutableStateOf(false) }
    val query = search.value
    TopAppBar(
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
        navigationIcon = {
            if (query != null) IconButton(onClick = { search.value = null }) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Close search")
            }
        },
        title = {
            if (query == null) Text("Voices")
            else TextField(
                value = query,
                onValueChange = { search.value = it },
                placeholder = { Text("Filter by name or language") },
                singleLine = true,
                colors = TextFieldDefaults.colors(
                    focusedContainerColor = MaterialTheme.colorScheme.surface,
                    unfocusedContainerColor = MaterialTheme.colorScheme.surface,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
        },
        actions = {
            if (query == null) IconButton(onClick = { search.value = "" }) { Icon(Icons.Filled.Search, contentDescription = "Search") }
            Box {
                IconButton(onClick = { menu = true }) { Icon(Icons.Filled.MoreVert, contentDescription = "More") }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    DropdownMenuItem(text = { Text("Refresh") }, onClick = { menu = false; model.refresh() })
                    DropdownMenuItem(text = { Text("Text-to-speech settings") }, onClick = { menu = false; openTtsSettings(context) })
                }
            }
        },
    )
}

@Composable
private fun Avatar(name: String, dim: Boolean = false) {
    Box(
        Modifier.size(40.dp).alpha(if (dim) 0.6f else 1f).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Text(name.take(1).uppercase(), color = MaterialTheme.colorScheme.onSecondaryContainer, style = MaterialTheme.typography.titleMedium)
    }
}

@Composable
private fun SectionLabel(text: String, trailing: String? = null) {
    Row(Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp)) {
        Text(text, color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.titleSmall)
        if (trailing != null) Text(" · $trailing", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium)
    }
}

private fun matches(query: String?, vararg fields: String) =
    query.isNullOrBlank() || fields.any { it.contains(query.trim(), ignoreCase = true) }

@Composable
fun VoicesScreen(model: MainModel, query: String?, modifier: Modifier) {
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) model.import(uri)
    }
    var family by rememberSaveable { mutableStateOf<String?>(null) }
    var deleting by remember { mutableStateOf<InstalledVoice?>(null) }
    val families = remember(model.catalog) {
        model.catalog.groupingBy { it.family }.eachCount().entries
            .sortedWith(compareByDescending<Map.Entry<String, Int>> { it.value }.thenBy { it.key }).map { it.key }
    }
    val installed = model.installed.filter { matches(query, it.name, model.languageOf(it), it.motherTongue) }
    val catalog = model.catalog.filter {
        (family == null || it.family == family) && matches(query, it.label, it.voice, it.language)
    }
    val mirrorHost = model.mirror.takeIf { it.isNotEmpty() }?.let { runCatching { java.net.URI(it).host }.getOrNull() ?: it }

    LazyColumn(modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 16.dp)) {
        item {
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Outlined.Storage, contentDescription = null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Spacer(Modifier.width(6.dp))
                Text(
                    "${size(model.bytesUsed)} in ${model.installed.size} voice${if (model.installed.size == 1) "" else "s"}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.weight(1f),
                )
                FilledTonalButton(onClick = { picker.launch(arrayOf("*/*")) }) {
                    Icon(Icons.Outlined.UploadFile, contentDescription = null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Import .exe")
                }
            }
        }
        item { SectionLabel("Installed") }
        if (model.installed.isEmpty()) {
            item {
                Text(
                    "No voices installed yet. Download one of the ${model.catalog.size} Loquendo 7 voices below, or import an installer .exe you already have.",
                    Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        items(installed, key = { "v-" + it.name }) { v -> InstalledRow(model, v) { deleting = v } }
        item {
            SectionLabel("Catalog · ${model.catalog.size} voices", mirrorHost?.let { "mirror $it" })
            LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                item { Chip("All", family == null) { family = null } }
                items(families) { f -> Chip(f, family == f) { family = if (family == f) null else f } }
            }
        }
        items(catalog, key = { "c-" + it.file }) { i -> CatalogRow(model, i) }
    }

    deleting?.let { v -> DeleteDialog(model, v, onDismiss = { deleting = null }) }
    model.importing?.let { ImportDialog(it, model::cancelImport) }
}

@Composable
private fun Chip(label: String, selected: Boolean, onClick: () -> Unit) {
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = { Text(label) },
        leadingIcon = if (selected) ({ Icon(Icons.Filled.Check, contentDescription = null, Modifier.size(18.dp)) }) else null,
        colors = FilterChipDefaults.filterChipColors(selectedContainerColor = MaterialTheme.colorScheme.primaryContainer),
    )
}

private val listColors @Composable get() = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface)

@Composable
private fun InstalledRow(model: MainModel, v: InstalledVoice, onDelete: () -> Unit) {
    val supporting: @Composable () -> Unit = when {
        v.unsupported != null -> ({ Text(v.unsupported, color = MaterialTheme.colorScheme.error) })
        v.languageMissing -> ({
            Text(
                "needs ${v.motherTongue} language data: add any ${model.languageOf(v)} voice or download it from the mirror",
                color = MaterialTheme.colorScheme.primary,
            )
        })
        else -> ({ Text("${model.languageOf(v)} · ${v.gender} · ${v.rate / 1000} kHz") })
    }
    ListItem(
        colors = listColors,
        leadingContent = { Avatar(v.name) },
        headlineContent = { Text(v.name) },
        supportingContent = supporting,
        trailingContent = {
            Row {
                if (v.languageMissing && v.unsupported == null) {
                    IconButton(onClick = { model.downloadLanguage(v) }) {
                        Icon(Icons.Outlined.CloudDownload, contentDescription = "Get ${v.motherTongue} language data", tint = MaterialTheme.colorScheme.primary)
                    }
                }
                IconButton(onClick = onDelete) { Icon(Icons.Outlined.Delete, contentDescription = "Delete ${v.name}") }
            }
        },
    )
}

@Composable
private fun CatalogRow(model: MainModel, i: Installer) {
    val state = model.downloads[i.file]
    val installedAs = model.installedFor(i)
    val parts = mutableListOf(i.language, i.gender, size(i.size))
    when {
        state is Download.Queued -> parts += "queued"
        state is Download.Running && state.unpacking -> parts += "unpacking"
        installedAs != null && !installedAs.name.equals(i.label, ignoreCase = true) -> parts += "installed as ${installedAs.name}"
        installedAs != null -> parts += "installed"
        i.unsupported != null -> parts += i.unsupported
    }
    ListItem(
        colors = listColors,
        modifier = if (state is Download.Failed) Modifier.clickable { model.download(i) } else Modifier,
        leadingContent = { Avatar(i.label) },
        headlineContent = { Text(i.label) },
        supportingContent = {
            Column {
                Text(parts.joinToString(" · "))
                when (state) {
                    is Download.Running -> if (!state.unpacking) {
                        Spacer(Modifier.height(6.dp))
                        val total = state.total.takeIf { it > 0 } ?: i.size
                        LinearProgressIndicator(
                            progress = { (state.done.toFloat() / total).coerceIn(0f, 1f) },
                            modifier = Modifier.fillMaxWidth(),
                            trackColor = MaterialTheme.colorScheme.primaryContainer,
                        )
                        Spacer(Modifier.height(4.dp))
                        Text(progressCaption(state.done, total, state.bytesPerSecond), style = MaterialTheme.typography.bodySmall)
                    } else {
                        Spacer(Modifier.height(6.dp))
                        LinearProgressIndicator(Modifier.fillMaxWidth(), trackColor = MaterialTheme.colorScheme.primaryContainer)
                    }
                    is Download.Failed -> Text("${state.message}. Tap to retry.", color = MaterialTheme.colorScheme.error)
                    else -> {}
                }
            }
        },
        trailingContent = {
            when {
                // unpacking runs in native code and can't stop halfway
                state is Download.Running && state.unpacking -> {}
                state is Download.Running || state is Download.Queued ->
                    IconButton(onClick = { model.cancel(i) }) { Icon(Icons.Filled.Close, contentDescription = "Cancel") }
                state is Download.Failed ->
                    IconButton(onClick = { model.download(i) }) { Icon(Icons.Filled.Refresh, contentDescription = "Retry", tint = MaterialTheme.colorScheme.error) }
                installedAs != null ->
                    Icon(Icons.Outlined.CheckCircle, contentDescription = "Installed", tint = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(12.dp))
                else -> IconButton(onClick = { model.download(i) }) {
                    Icon(
                        Icons.Outlined.FileDownload,
                        contentDescription = "Download ${i.label}",
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.alpha(if (i.unsupported != null) 0.38f else 1f),
                    )
                }
            }
        },
    )
}

private fun progressCaption(done: Long, total: Long, bytesPerSecond: Double): String {
    val mb = { b: Long -> "%.1f".format(Locale.ROOT, b / 1e6) }
    val base = "${mb(done)} of ${mb(total)} MB"
    if (bytesPerSecond <= 0) return base
    val left = ((total - done) / bytesPerSecond).toLong().coerceAtLeast(0)
    val eta = if (left >= 60) "${left / 60} min left" else "$left s left"
    return "$base · %.1f MB/s · $eta".format(Locale.ROOT, bytesPerSecond / 1e6)
}

@Composable
private fun DeleteDialog(model: MainModel, v: InstalledVoice, onDismiss: () -> Unit) {
    var bytes by remember(v) { mutableStateOf<Long?>(null) }
    androidx.compose.runtime.LaunchedEffect(v) {
        bytes = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) { model.store.bytesOf(v) }
    }
    val sharer = model.installed.firstOrNull { it.name != v.name && it.languageFolder != null && it.languageFolder == v.languageFolder }
    val language = v.languageFolder?.let { f ->
        if (sharer != null) "$f language data stays, because ${sharer.name} uses it too. " else "$f language data stays. "
    } ?: ""
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Outlined.Delete, contentDescription = null) },
        title = { Text("Delete ${v.name}?") },
        text = { Text("Frees ${bytes?.let { size(it) } ?: "…"}. ${language}You can download ${v.name} again from the catalog.") },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
        confirmButton = {
            TextButton(onClick = { model.delete(v); onDismiss() }) {
                Text("Delete", color = MaterialTheme.colorScheme.error)
            }
        },
    )
}

@Composable
private fun ImportDialog(state: Import, onCancel: () -> Unit) {
    AlertDialog(
        onDismissRequest = {},
        title = { Text("Importing ${state.name.replace(" - ", " – ")}") },
        text = {
            Column {
                Text("Unpacking the installer and copying the voice to quoquendo's storage. This takes a few seconds.")
                Spacer(Modifier.height(20.dp))
                if (state.unpacking || state.total <= 0) {
                    LinearProgressIndicator(Modifier.fillMaxWidth(), trackColor = MaterialTheme.colorScheme.primaryContainer)
                } else {
                    LinearProgressIndicator(
                        progress = { (state.done.toFloat() / state.total).coerceIn(0f, 1f) },
                        modifier = Modifier.fillMaxWidth(),
                        trackColor = MaterialTheme.colorScheme.primaryContainer,
                    )
                }
                Spacer(Modifier.height(8.dp))
                val caption = when {
                    state.unpacking -> "Unpacking ${state.name}…"
                    state.total > 0 -> "${state.name} · %.1f of %.1f MB".format(Locale.ROOT, state.done / 1e6, state.total / 1e6)
                    else -> "${state.name} · %.1f MB".format(Locale.ROOT, state.done / 1e6)
                }
                Text(caption, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        },
        confirmButton = { TextButton(onClick = onCancel, enabled = !state.unpacking) { Text("Cancel") } },
    )
}
