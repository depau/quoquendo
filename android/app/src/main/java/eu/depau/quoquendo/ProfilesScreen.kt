package eu.depau.quoquendo

import androidx.activity.compose.BackHandler
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.FileDownload
import androidx.compose.material.icons.outlined.MoreVert
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import org.json.JSONObject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfilesTopBar(go: (Screen) -> Unit) {
    TopAppBar(
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
        title = { Text("Profiles") },
        navigationIcon = {
            IconButton(onClick = { go(Screen.Settings) }) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
            }
        },
    )
}

@Composable
fun ProfilesScreen(model: MainModel, go: (Screen) -> Unit, modifier: Modifier) {
    var editing by rememberSaveable { mutableStateOf<String?>(null) }
    var choosingApp by rememberSaveable { mutableStateOf(false) }
    BackHandler(enabled = editing != null || choosingApp) { editing = null; choosingApp = false }
    val context = LocalContext.current
    val import = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) {
            val json = runCatching { context.contentResolver.openInputStream(uri)?.bufferedReader()?.readText().orEmpty() }.getOrDefault("")
            model.message(model.importProfile(json) ?: "Profile imported")
        }
    }
    val name = editing
    if (name != null) {
        ProfileEditor(model, name, onBack = { editing = null }, onOpen = { editing = it }, onDelete = { editing = null }, modifier)
    } else if (choosingApp) {
        ChooseApp(model, onDone = { choosingApp = false }, modifier)
    } else {
        Column(modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            Text(
                "Tap a profile to edit it; tap its radio button to use it.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            )
            for (p in model.profiles) {
                ListItem(
                    colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
                    modifier = Modifier.clickable { editing = p.name },
                    leadingContent = {
                        RadioButton(selected = model.activeProfile.equals(p.name, ignoreCase = true), onClick = { model.useProfile(p.name) })
                    },
                    headlineContent = { Text(p.name) },
                    supportingContent = { Text(if (p.builtin) "Built-in, read-only" else "Custom") },
                    trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) },
                )
            }
            Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { import.launch(arrayOf("application/json")) }) {
                    Icon(Icons.Outlined.UploadFile, contentDescription = null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Import profile")
                }
            }
            Section("Apps with their own profile")
            for (app in model.appProfiles.keys.sorted()) {
                AppProfileRow(model, app, appLabel(app))
            }
            ListItem(
                colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.clickable { choosingApp = true },
                leadingContent = { Icon(Icons.Outlined.Add, contentDescription = null) },
                headlineContent = { Text("Choose an app") },
            )
        }
    }
}

@Composable
private fun ProfileEditor(model: MainModel, name: String, onBack: () -> Unit, onOpen: (String) -> Unit, onDelete: () -> Unit, modifier: Modifier) {
    val entry = model.profiles.firstOrNull { it.name.equals(name, ignoreCase = true) }
    if (entry == null) {
        onBack()
        return
    }
    var draft by remember(entry.name) { mutableStateOf(entry.json) }
    var tryText by remember { mutableStateOf("") }
    var menu by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { uri ->
        if (uri != null) {
            runCatching { context.contentResolver.openOutputStream(uri)?.use { it.write(draft.toByteArray()) } }
                .onSuccess { model.message("Profile exported") }
                .onFailure { error = it.message }
        }
    }
    val o = remember(draft) { runCatching { JSONObject(draft) }.getOrElse { JSONObject() } }
    fun edit(block: (JSONObject) -> Unit) {
        val next = runCatching { JSONObject(draft) }.getOrElse { JSONObject() }
        block(next)
        draft = next.toString()
        error = null
    }
    fun save() {
        if (entry.builtin) return
        error = model.saveProfile(entry.name, draft)
        if (error == null) onBack()
    }

    Column(modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
        Row(Modifier.fillMaxWidth().padding(start = 8.dp, end = 8.dp, top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = { if (entry.builtin) onBack() else save() }) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
            }
            Text(if (entry.builtin) entry.name else "Edit profile", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            if (!entry.builtin) {
                TextButton(onClick = { save() }) { Text("Save") }
            }
            IconButton(onClick = { menu = true }) { Icon(Icons.Outlined.MoreVert, contentDescription = "More") }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                DropdownMenuItem(text = { Text("Duplicate") }, onClick = {
                    menu = false
                    model.duplicateProfile(entry.name)?.let(onOpen)
                })
                if (!entry.builtin) {
                    DropdownMenuItem(text = { Text("Export") }, onClick = { menu = false; export.launch("${entry.name}.json") })
                    DropdownMenuItem(text = { Text("Delete") }, onClick = { menu = false; confirmDelete = true })
                }
            }
        }
        if (entry.builtin) {
            Text(
                "Built-in profiles can't be changed. Duplicate one to edit it.",
                color = MaterialTheme.colorScheme.primary,
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            )
        }
        if (error != null) Text(error!!, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 16.dp))

        Column(Modifier.padding(horizontal = 16.dp).then(if (entry.builtin) Modifier else Modifier)) {
            OutlinedTextField(
                value = o.optString("name"),
                onValueChange = { edit { p -> p.put("name", it) } },
                label = { Text("Name") },
                enabled = !entry.builtin,
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            Text("Try it", style = MaterialTheme.typography.titleSmall)
            OutlinedTextField(
                value = tryText,
                onValueChange = { tryText = it },
                singleLine = true,
                placeholder = { Text("22/11/26, €10, @anna") },
                modifier = Modifier.fillMaxWidth(),
            )
            if (tryText.isNotBlank()) {
                val applied = remember(tryText, draft) {
                    runCatching { JSONObject(Native.applyProfile(tryText, draft, null)) }.getOrNull()
                }
                Text(applied?.optString("text").orEmpty(), color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(top = 4.dp))
                val warnings = applied?.optJSONArray("warnings") ?: JSONArray()
                for (i in 0 until warnings.length()) {
                    Text(warnings.optString(i), color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                }
            }
        }

        Section("Dates & numbers")
        Choice("Dates", listOf("full", "numeric", "verbatim", "as_written"), listOf("Day, month, year", "Numbers", "As written", "Leave to engine"), o.optString("dates", "as_written"), !entry.builtin) { v -> edit { it.put("dates", v) } }
        Choice("Two-digit years", listOf("next_ten", "original"), listOf("Next ten years", "Original cutoff"), o.optString("year", "original"), !entry.builtin) { v -> edit { it.put("year", v) } }
        Choice("Ranges", listOf("words", "numbers", "as_written"), listOf("From 10 to 15", "10 15", "Leave to engine"), o.optString("ranges", "as_written"), !entry.builtin) { v -> edit { it.put("ranges", v) } }
        Toggle("Formulas", o.optBoolean("math"), !entry.builtin) { v -> edit { it.put("math", v) } }

        Section("Text & web")
        for ((key, label) in listOf(
            "inclusive" to "Inclusive Italian (ə, ɜ, *)",
            "units" to "Units",
            "handles" to "Handles & hashtags",
            "federated" to "Fediverse, Matrix & Threads",
            "emails" to "Emails",
            "links" to "Links",
            "cashtags" to "Cashtags",
            "currencies" to "Currencies",
            "crypto" to "Crypto",
        )) {
            Toggle(label, o.optBoolean(key), !entry.builtin) { v -> edit { it.put(key, v) } }
        }
        Choice("Emoji", listOf("describe", "skip"), listOf("Describe", "Skip"), o.optString("emoji", "skip"), !entry.builtin) { v -> edit { it.put("emoji", v) } }

        Section("Audio")
        Choice("Joining", listOf("smooth", "original"), listOf("Smoothed", "Like the original"), o.optString("joins", "original"), !entry.builtin) { v -> edit { it.put("joins", v) } }

        Section("Easter eggs")
        Toggle("Italian station announcement", o.optBoolean("station_warning"), !entry.builtin) { v -> edit { it.put("station_warning", v) } }

        Section("Rules")
        OutlinedTextField(
            value = o.optString("say_before"),
            onValueChange = { edit { p -> p.put("say_before", it) } },
            label = { Text("Say before") },
            enabled = !entry.builtin,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        )
        OutlinedTextField(
            value = o.optString("say_after"),
            onValueChange = { edit { p -> p.put("say_after", it) } },
            label = { Text("Say after") },
            enabled = !entry.builtin,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        )
        OutlinedTextField(
            value = o.optString("rules_language"),
            onValueChange = { edit { p -> if (it.isBlank()) p.remove("rules_language") else p.put("rules_language", it) } },
            label = { Text("Language (blank = all)") },
            enabled = !entry.builtin,
            singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        )

        Replacements(o, !entry.builtin, ::edit)
        Spacer(Modifier.height(24.dp))
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("Delete profile") },
            text = { Text("Delete “${entry.name}”? This cannot be undone.") },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text("Cancel") } },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    val e = model.deleteProfile(entry.name)
                    if (e == null) onDelete() else error = e
                }) { Text("Delete", color = MaterialTheme.colorScheme.error) }
            },
        )
    }
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
private fun Toggle(label: String, checked: Boolean, enabled: Boolean, onChange: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth().clickable(enabled = enabled) { onChange(!checked) }.padding(horizontal = 16.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
        Switch(checked = checked, onCheckedChange = onChange, enabled = enabled)
    }
}

@Composable
private fun Choice(title: String, values: List<String>, labels: List<String>, selected: String, enabled: Boolean, onChange: (String) -> Unit) {
    Text(title, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(start = 16.dp, top = 8.dp))
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        values.forEachIndexed { i, value ->
            FilterChip(
                selected = selected == value,
                onClick = { if (enabled) onChange(value) },
                enabled = enabled,
                label = { Text(labels[i]) },
            )
        }
    }
}

@Composable
private fun Replacements(o: JSONObject, enabled: Boolean, edit: ((JSONObject) -> Unit) -> Unit) {
    val rules = o.optJSONArray("replacements") ?: JSONArray()
    Section("Replacements")
    for (i in 0 until rules.length()) {
        val r = rules.optJSONObject(i) ?: continue
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
            Switch(checked = r.optBoolean("enabled", true), enabled = enabled, onCheckedChange = { v ->
                edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).put("enabled", v) }
            })
            OutlinedTextField(
                value = r.optString("find"),
                onValueChange = { v -> edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).put("find", v) } },
                label = { Text("Find") },
                enabled = enabled,
                singleLine = true,
                modifier = Modifier.weight(1f).padding(horizontal = 4.dp),
            )
            OutlinedTextField(
                value = r.optString("replace"),
                onValueChange = { v -> edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).put("replace", v) } },
                label = { Text("Replace with") },
                enabled = enabled,
                singleLine = true,
                modifier = Modifier.weight(1f).padding(horizontal = 4.dp),
            )
            IconButton(enabled = enabled, onClick = { edit { p -> p.optJSONArray("replacements")!!.remove(i) } }) {
                Icon(Icons.Outlined.Delete, contentDescription = "Delete replacement")
            }
        }
        Row(Modifier.fillMaxWidth().padding(horizontal = 72.dp, vertical = 0.dp), verticalAlignment = Alignment.CenterVertically) {
            Toggle("Regex", r.optBoolean("regex"), enabled) { v -> edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).put("regex", v) } }
            Toggle("Match case", r.optBoolean("match_case", true), enabled) { v -> edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).put("match_case", v) } }
        }
        OutlinedTextField(
            value = r.optString("language"),
            onValueChange = { v -> edit { p -> p.optJSONArray("replacements")!!.getJSONObject(i).also { if (v.isBlank()) it.remove("language") else it.put("language", v) } } },
            label = { Text("Language (blank = all)") },
            enabled = enabled,
            singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 72.dp),
        )
    }
    if (enabled) {
        Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.Center) {
            OutlinedButton(onClick = {
                edit { p ->
                    val arr = p.optJSONArray("replacements") ?: JSONArray().also { p.put("replacements", it) }
                    arr.put(JSONObject().put("find", "").put("replace", "").put("regex", false).put("match_case", true).put("enabled", true))
                }
            }) {
                Icon(Icons.Outlined.Add, contentDescription = null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text("Add replacement")
            }
        }
    }
}
