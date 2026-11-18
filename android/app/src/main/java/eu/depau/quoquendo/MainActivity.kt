package eu.depau.quoquendo

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.speech.tts.TextToSpeech
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

enum class Screen(val title: String) { Voices("Voices"), Try("Try it"), Settings("Settings") }

class MainActivity : ComponentActivity() {
    private val model: MainModel by viewModels()
    private var screen = mutableStateOf(Screen.Try)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        route(intent)
        setContent {
            MaterialTheme(colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme()) {
                App(model, screen.value) { screen.value = it }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        route(intent)
    }

    override fun onResume() {
        super.onResume()
        model.refresh()
    }

    private fun route(intent: Intent?) {
        if (intent?.action == TextToSpeech.Engine.ACTION_INSTALL_TTS_DATA || intent?.getStringExtra("screen") == "voices") {
            screen.value = Screen.Voices
        }
    }
}

@Composable
fun App(model: MainModel, screen: Screen, go: (Screen) -> Unit) {
    Scaffold(
        bottomBar = {
            NavigationBar {
                Screen.entries.forEach { s ->
                    NavigationBarItem(selected = s == screen, onClick = { go(s) }, icon = {}, label = { Text(s.title) })
                }
            }
        },
    ) { pad ->
        Column(Modifier.padding(pad).fillMaxSize()) {
            model.messages.firstOrNull()?.let {
                Text(
                    it,
                    Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.secondaryContainer).padding(12.dp),
                    color = MaterialTheme.colorScheme.onSecondaryContainer,
                )
            }
            when (screen) {
                Screen.Voices -> VoicesScreen(model)
                Screen.Try -> TryScreen(model, go)
                Screen.Settings -> SettingsScreen(model)
            }
        }
    }
}

private fun mb(bytes: Long) = "%.1f MB".format(bytes / 1_000_000.0)

@Composable
fun VoicesScreen(model: MainModel) {
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) model.import(uri)
    }
    val storage = remember(model.installed) { model.store.root.walkTopDown().filter { it.isFile }.sumOf { it.length() } }
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        item {
            Row(Modifier.fillMaxWidth().padding(top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Installed voices", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
                Button(onClick = { picker.launch(arrayOf("*/*")) }) { Text("Import .exe") }
            }
            Text("Storage used: ${mb(storage)}", style = MaterialTheme.typography.bodySmall)
            model.tasks["import"]?.let { TaskRow(it) }
        }
        if (model.installed.isEmpty()) {
            item { Text("No voices yet: download one below or import an installer.") }
        }
        items(model.installed, key = { "v-" + it.name }) { v ->
            Card(Modifier.fillMaxWidth()) {
                Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(v.name, fontWeight = FontWeight.Bold)
                        Text("${v.description} · ${v.gender} · ${v.locales.joinToString { it.toLanguageTag() }}", style = MaterialTheme.typography.bodySmall)
                        (v.problem ?: v.warning)?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                    }
                    TextButton(onClick = { model.delete(v) }) { Text("Delete") }
                }
            }
        }
        item {
            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            Text("Loquendo 7 voice pack", style = MaterialTheme.typography.titleLarge)
            Text(
                if (model.mirror.isNotEmpty()) "Downloads from ${model.mirror}, falling back to archive.org"
                else "Downloads from archive.org",
                style = MaterialTheme.typography.bodySmall,
            )
        }
        items(model.catalog, key = { "c-" + it.file }) { i ->
            val task = model.tasks[i.file]
            Card(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(i.label, fontWeight = FontWeight.Bold)
                            val voice = if (i.voice != i.label) " (voice ${i.voice})" else ""
                            Text("${i.language} · ${i.gender} · ${mb(i.size)}$voice", style = MaterialTheme.typography.bodySmall)
                        }
                        when {
                            task != null -> TextButton(onClick = { model.cancel(i) }) { Text("Cancel") }
                            model.isInstalled(i) -> Text("Installed", style = MaterialTheme.typography.labelMedium)
                            else -> OutlinedButton(onClick = { model.download(i) }) { Text("Download") }
                        }
                    }
                    task?.let { TaskRow(it) }
                }
            }
        }
        item { Spacer(Modifier.height(16.dp)) }
    }
}

@Composable
fun TaskRow(t: Task) {
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        if (t.unpacking) {
            Text("Unpacking…", style = MaterialTheme.typography.bodySmall)
            LinearProgressIndicator(Modifier.fillMaxWidth())
        } else if (t.total > 0) {
            Text("${t.title}: ${mb(t.done)} of ${mb(t.total)}", style = MaterialTheme.typography.bodySmall)
            LinearProgressIndicator(progress = { (t.done.toFloat() / t.total).coerceIn(0f, 1f) }, modifier = Modifier.fillMaxWidth())
        } else {
            Text("${t.title}: ${mb(t.done)}", style = MaterialTheme.typography.bodySmall)
            LinearProgressIndicator(Modifier.fillMaxWidth())
        }
    }
}

/** A TextToSpeech client bound to quoquendo's own engine, released with the composition. */
@Composable
fun rememberTts(context: Context): Pair<TextToSpeech?, Boolean> {
    var ready by remember { mutableStateOf(false) }
    val tts = remember {
        TextToSpeech(context.applicationContext, { status -> ready = status == TextToSpeech.SUCCESS }, "eu.depau.quoquendo")
    }
    DisposableEffect(tts) { onDispose { tts.shutdown() } }
    return tts to ready
}

@Composable
fun TryScreen(model: MainModel, go: (Screen) -> Unit) {
    val context = LocalContext.current
    val (tts, ready) = rememberTts(context)
    val voices = model.installed
    var chosen by rememberSaveable { mutableStateOf<String?>(null) }
    val voice = voices.firstOrNull { it.name == chosen } ?: voices.firstOrNull { it.speakable }
    var text by rememberSaveable { mutableStateOf("") }
    var showCurtain by rememberSaveable { mutableStateOf(false) }
    var menu by remember { mutableStateOf(false) }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        if (voices.isEmpty()) {
            Text("No voices installed.")
            Button(onClick = { go(Screen.Voices) }) { Text("Get a voice") }
            return@Column
        }
        Box {
            OutlinedButton(onClick = { menu = true }, modifier = Modifier.fillMaxWidth()) {
                Text(voice?.let { "${it.name} · ${it.description}" } ?: "Pick a voice")
            }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                voices.forEach { v ->
                    DropdownMenuItem(
                        text = { Text(if (v.speakable) v.name else "${v.name} (unsupported)") },
                        onClick = { chosen = v.name; menu = false },
                    )
                }
            }
        }
        voice?.let { v -> (v.problem ?: v.warning)?.let { Text(it, color = MaterialTheme.colorScheme.error) } }
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            label = { Text("Text") },
            modifier = Modifier.fillMaxWidth(),
            minLines = 3,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                enabled = ready && voice != null && voice.speakable && text.isNotBlank(),
                onClick = {
                    val v = voice ?: return@Button
                    val t = tts ?: return@Button
                    val locale = v.locales.firstOrNull()
                    t.voices?.firstOrNull { locale != null && it.name == VoiceStore.ttsVoiceName(v.name, locale) }
                        ?.let { t.voice = it }
                        ?: locale?.let { t.language = it }
                    t.speak(text, TextToSpeech.QUEUE_FLUSH, null, "qq-try")
                    if (showCurtain) model.analyse(v.name, text)
                },
            ) { Text("Speak") }
            OutlinedButton(enabled = ready, onClick = { tts?.stop() }) { Text("Stop") }
            if (text.isEmpty() && voice != null) {
                TextButton(onClick = { text = sampleFor(voice) }) { Text("Sample text") }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Switch(checked = showCurtain, onCheckedChange = {
                showCurtain = it
                if (it && voice != null && voice.speakable && text.isNotBlank()) model.analyse(voice.name, text)
            })
            Spacer(Modifier.width(8.dp))
            Text("Show how it is made")
        }
        if (showCurtain) model.analysis?.let { Curtain(it) }
    }
}

private fun sampleFor(v: InstalledVoice): String {
    val lang = v.locales.firstOrNull()?.language ?: ""
    return when (lang) {
        "it" -> "Ciao, mi chiamo ${v.name}. Come stai oggi?"
        "fr" -> "Bonjour, je m'appelle ${v.name}. Comment allez-vous ?"
        "de" -> "Hallo, ich heiße ${v.name}. Wie geht es dir?"
        "es" -> "Hola, me llamo ${v.name}. ¿Cómo estás hoy?"
        "pt" -> "Olá, o meu nome é ${v.name}. Como está?"
        else -> "Hello, my name is ${v.name}. How are you today?"
    }
}

private val known = Color(0xFF4CAF50)
private val unknown = Color(0xFFE53935)
private val hardCut = Color(0xFFE6AA5A)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun Curtain(a: Analysis) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Sounds", style = MaterialTheme.typography.titleMedium)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            a.phrases.forEach { p ->
                p.phones.forEachIndexed { i, ph ->
                    if (ph.wordStart && i > 0) Text("|", color = MaterialTheme.colorScheme.outline, fontFamily = FontFamily.Monospace)
                    Text(ph.sym, color = if (ph.known) known else unknown, fontFamily = FontFamily.Monospace)
                }
                if (p.pauseMs > 0) {
                    val mood = when (p.mood) { "I" -> " ?"; "E" -> " !"; else -> "" }
                    Text("· ${p.pauseMs} ms$mood ·", color = MaterialTheme.colorScheme.outline, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
        if (a.missing.isNotEmpty()) {
            Text("Never recorded by this voice, skipped: ${a.missing.joinToString()}", color = hardCut)
        }
        if (a.envelope.isNotEmpty()) {
            val color = MaterialTheme.colorScheme.primary
            Canvas(Modifier.fillMaxWidth().height(64.dp)) {
                val w = size.width / a.envelope.size
                val mid = size.height / 2
                a.envelope.forEachIndexed { i, e ->
                    val h = e * mid
                    drawLine(color, Offset(i * w, mid - h), Offset(i * w, mid + h), strokeWidth = w.coerceAtLeast(1f))
                }
            }
        }
        Text("Recorded pieces (${a.pieces.size})", style = MaterialTheme.typography.titleMedium)
        a.pieces.take(300).forEach { p ->
            Column(Modifier.fillMaxWidth()) {
                Text(p.covers, color = if (p.hard) hardCut else known, fontFamily = FontFamily.Monospace)
                Text(
                    "#${p.unit} ${p.sentence.take(40)} · %.2f s · %.0f ms · %d Hz".format(p.atS, p.lenMs, p.pitch),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.outline,
                )
            }
        }
    }
}

@Composable
fun SettingsScreen(model: MainModel) {
    val context = LocalContext.current
    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Text-to-speech", style = MaterialTheme.typography.titleLarge)
        Text("To let other apps speak with these voices, choose quoquendo as the preferred engine in the system text-to-speech settings.")
        Button(onClick = {
            try {
                context.startActivity(Intent("com.android.settings.TTS_SETTINGS"))
            } catch (_: ActivityNotFoundException) {
                model.say("This device has no text-to-speech settings screen")
            }
        }) { Text("Open text-to-speech settings") }
        HorizontalDivider()
        Text("Voice mirror: ${model.mirror.ifEmpty { "none (archive.org only)" }}", style = MaterialTheme.typography.bodySmall)
        Text("Voice store: ${model.store.root.path}", style = MaterialTheme.typography.bodySmall)
    }
}
