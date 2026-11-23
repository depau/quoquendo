package eu.depau.quoquendo

import android.content.Intent
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.graphics.drawable.toBitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** An app's launcher icon, or nothing when it can't be read. */
@Composable
fun appIcon(app: String): ImageBitmap? {
    val pm = LocalContext.current.packageManager
    return remember(app) { runCatching { pm.getApplicationIcon(app).toBitmap(96, 96).asImageBitmap() }.getOrNull() }
}

/** An app's name, or its package when it can't be read. */
@Composable
fun appLabel(app: String): String {
    val pm = LocalContext.current.packageManager
    return remember(app) { runCatching { pm.getApplicationLabel(pm.getApplicationInfo(app, 0)).toString() }.getOrDefault(app) }
}

/** The app's icon in a 40 dp circle, or its initial. */
@Composable
private fun AppAvatar(app: String, label: String) {
    val icon = appIcon(app)
    if (icon != null) Image(icon, contentDescription = null, Modifier.size(40.dp))
    else Box(Modifier.size(40.dp).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape), contentAlignment = Alignment.Center) {
        Text(label.take(1).uppercase(), color = MaterialTheme.colorScheme.onSecondaryContainer)
    }
}

private const val ACTIVE = "Use active profile"

/** An app and the profile its speech is read with; tapping it chooses another. */
@Composable
fun AppProfileRow(model: MainModel, app: String, label: String, headline: String = label, trailing: @Composable (() -> Unit)? = null) {
    var choosing by remember { mutableStateOf(false) }
    ListItem(
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier.clickable { choosing = true },
        leadingContent = { AppAvatar(app, label) },
        headlineContent = { Text(headline) },
        supportingContent = { Text(model.appProfiles[app] ?: ACTIVE) },
        trailingContent = trailing ?: { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) },
    )
    if (choosing) AppProfileDialog(model, app, label) { choosing = false }
}

/** "Profile for <app>": the active profile, or one of its own. */
@Composable
private fun AppProfileDialog(model: MainModel, app: String, label: String, onDismiss: () -> Unit) {
    val own = model.appProfiles[app]
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Profile for $label") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                @Composable
                fun Option(title: String, detail: String?, selected: Boolean, name: String?) {
                    Row(
                        Modifier.fillMaxWidth().clickable { model.setAppProfile(app, name); onDismiss() }.padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = selected, onClick = null)
                        Spacer(Modifier.width(16.dp))
                        Column {
                            Text(title, style = MaterialTheme.typography.bodyLarge)
                            if (detail != null) Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
                Option(ACTIVE, model.activeProfile, own == null, null)
                for (p in model.profiles) Option(p.name, null, own == p.name, p.name)
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}

/** Every app with a launcher icon, apps with their own profile first. */
@Composable
fun ChooseApp(model: MainModel, onDone: () -> Unit, modifier: Modifier) {
    val context = LocalContext.current
    val apps by produceState(emptyList<Pair<String, String>>()) {
        value = withContext(Dispatchers.IO) {
            val pm = context.packageManager
            pm.queryIntentActivities(Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER), 0)
                .map { it.activityInfo.packageName to it.loadLabel(pm).toString() }
                .filter { it.first != context.packageName }
                .distinctBy { it.first }
                .sortedBy { it.second.lowercase() }
        }
    }
    val sorted = apps.sortedBy { it.first !in model.appProfiles }
    Column(modifier.fillMaxSize()) {
        Text("Choose an app", style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(16.dp))
        LazyColumn(Modifier.weight(1f)) {
            items(sorted, key = { it.first }) { (app, label) ->
                AppProfileRow(model, app, label) {
                    if (app in model.appProfiles) Icon(Icons.Filled.Check, contentDescription = "Has its own profile", tint = MaterialTheme.colorScheme.primary)
                }
            }
        }
        Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.Center) {
            Button(onClick = onDone, modifier = Modifier.fillMaxWidth()) { Text("Done") }
        }
    }
}
