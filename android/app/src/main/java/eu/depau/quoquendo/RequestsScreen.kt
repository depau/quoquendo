package eu.depau.quoquendo

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.util.Locale
import kotlin.math.roundToInt

/** "kept for 7 days", or "kept until cleared". */
fun retention(days: Int): String = when (days) {
    0 -> "kept until cleared"
    1 -> "kept for 1 day"
    else -> "kept for $days days"
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RequestsTopBar(model: MainModel, go: (Screen) -> Unit, search: MutableState<String?>) {
    var menu by remember { mutableStateOf(false) }
    var confirmClear by remember { mutableStateOf(false) }
    val query = search.value
    BackHandler(enabled = query != null) { search.value = null }
    TopAppBar(
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
        navigationIcon = {
            IconButton(onClick = { if (query != null) search.value = null else go(Screen.Settings) }) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = if (query != null) "Close search" else "Back")
            }
        },
        title = {
            if (query == null) Text("Speech requests")
            else TextField(
                value = query,
                onValueChange = { search.value = it },
                placeholder = { Text("Search requests") },
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
                    Text(
                        "Keep requests",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    )
                    for (days in SpeechLog.RETENTION) {
                        DropdownMenuItem(
                            leadingIcon = { RadioButton(selected = model.requestDays == days, onClick = null) },
                            text = { Text(if (days == 0) "Until cleared" else if (days == 1) "1 day" else "$days days") },
                            onClick = { menu = false; model.updateRequestDays(days) },
                        )
                    }
                    HorizontalDivider()
                    DropdownMenuItem(
                        text = { Text("Clear all") },
                        enabled = model.requests.isNotEmpty(),
                        onClick = { menu = false; confirmClear = true },
                    )
                }
            }
        },
    )
    if (confirmClear) {
        AlertDialog(
            onDismissRequest = { confirmClear = false },
            title = { Text("Clear all speech requests?") },
            text = { Text("The ${model.requests.size} saved requests are deleted from this device.") },
            dismissButton = { TextButton(onClick = { confirmClear = false }) { Text("Cancel") } },
            confirmButton = { TextButton(onClick = { model.clearRequests(); confirmClear = false }) { Text("Clear") } },
        )
    }
}

@Composable
fun RequestsScreen(model: MainModel, query: String?, modifier: Modifier) {
    LaunchedEffect(Unit) { model.refreshRequests() }
    var picked by rememberSaveable { mutableStateOf<String?>(null) }
    // the apps, most recently heard first
    val apps = model.requests.asReversed().distinctBy { it.app }
    // an app whose requests were all cleared or expired is no filter
    val filter = picked?.takeIf { p -> apps.any { it.app == p } }
    val q = query.orEmpty().trim()
    val shown = model.requests.asReversed().filter { (filter == null || it.app == filter) && (q.isEmpty() || it.text.contains(q, ignoreCase = true)) }

    Column(modifier.fillMaxSize()) {
        if (apps.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                FilterChip(selected = filter == null, onClick = { picked = null }, label = { Text("All apps") })
                for (a in apps) {
                    val icon = appIcon(a.app)
                    FilterChip(
                        selected = filter == a.app,
                        onClick = { picked = if (filter == a.app) null else a.app },
                        label = { Text(a.label) },
                        leadingIcon = icon?.let { { Image(it, contentDescription = null, Modifier.size(18.dp)) } },
                    )
                }
            }
        }
        // an app the engine can name gets its own profile here
        apps.firstOrNull { it.app == filter }?.takeIf { !it.app.startsWith("uid ") }?.let { a ->
            AppProfileRow(model, a.app, a.label, headline = "Profile for ${a.label}")
        }
        if (shown.isEmpty()) {
            Text(
                when {
                    model.requests.isNotEmpty() -> "No requests match"
                    model.saveRequests -> "No requests yet. They appear here when apps speak with Quoquendo."
                    else -> "Turn on Save speech requests in Settings to see what apps ask Quoquendo to say."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth().padding(32.dp),
            )
            return@Column
        }
        val zone = ZoneId.systemDefault()
        val today = LocalDate.now(zone)
        val dayFormat = DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM)
        val timeFormat = DateTimeFormatter.ofLocalizedTime(FormatStyle.SHORT)
        val days = shown.groupBy { Instant.ofEpochMilli(it.time).atZone(zone).toLocalDate() }
        LazyColumn(Modifier.fillMaxSize()) {
            for ((day, entries) in days) {
                item(key = day.toString()) {
                    Text(
                        when (day) {
                            today -> "Today"
                            today.minusDays(1) -> "Yesterday"
                            else -> day.format(dayFormat)
                        },
                        color = MaterialTheme.colorScheme.primary,
                        style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 4.dp),
                    )
                }
                items(entries) { r -> Request(r, Instant.ofEpochMilli(r.time).atZone(zone).format(timeFormat), showApp = filter == null && apps.size > 1) }
            }
        }
    }
}

@Composable
private fun Request(r: SpeechRequest, time: String, showApp: Boolean) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp)) {
        SelectionContainer { Text(r.text, style = MaterialTheme.typography.bodyLarge) }
        Row(Modifier.padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            val meta = listOfNotNull(time, r.label.takeIf { showApp }, r.language.takeIf { it.isNotEmpty() }?.let(::languageName))
            Text(meta.joinToString(" · "), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            r.pitch?.let { Tag("Pitch ${"%.2f".format(Locale.ROOT, it / 100.0).trimEnd('0').trimEnd('.')}×") }
            r.volume?.let { Tag("Volume ${(it * 100).roundToInt()}%") }
        }
    }
    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
}

@Composable
private fun Tag(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.onSecondaryContainer,
        modifier = Modifier
            .background(MaterialTheme.colorScheme.secondaryContainer, RoundedCornerShape(4.dp))
            .padding(horizontal = 6.dp, vertical = 2.dp),
    )
}
