package eu.depau.quoquendo

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.automirrored.outlined.VolumeUp
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Save
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TryTopBar(model: MainModel) {
    val context = LocalContext.current
    var menu by remember { mutableStateOf(false) }
    TopAppBar(
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface),
        title = { Text("Try it") },
        actions = {
            IconButton(onClick = { menu = true }) { Icon(Icons.Filled.MoreVert, contentDescription = "More") }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                DropdownMenuItem(text = { Text("Text-to-speech settings") }, onClick = { menu = false; openTtsSettings(context) })
            }
        },
    )
}

private val tryCaption = "Sounds, recorded pieces and the waveform"

/** `Roberto — Italian (Male)` */
private fun MainModel.title(v: InstalledVoice) = "${v.name} — ${languageOf(v)} (${v.gender})"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TryScreen(model: MainModel, go: (Screen) -> Unit, modifier: Modifier) {
    val context = LocalContext.current
    var bannerDismissed by rememberSaveable { mutableStateOf(false) }
    val voice = model.selectedVoice()
    // an empty text box starts with the voice's own sentence
    androidx.compose.runtime.LaunchedEffect(voice?.name) {
        if (voice != null && model.text.isBlank()) model.updateText(voice.demoSentence)
    }
    Column(
        modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Spacer(Modifier.height(4.dp))
        if (!model.isDefaultEngine && !bannerDismissed) {
            Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer), shape = RoundedCornerShape(16.dp)) {
                Column(Modifier.padding(start = 16.dp, end = 8.dp, top = 16.dp, bottom = 4.dp)) {
                    Row {
                        Icon(Icons.Outlined.Info, contentDescription = null, Modifier.padding(top = 2.dp).size(20.dp))
                        Spacer(Modifier.width(12.dp))
                        Column {
                            Text("Quoquendo isn't your default text-to-speech engine", style = MaterialTheme.typography.titleSmall)
                            Spacer(Modifier.height(4.dp))
                            Text(
                                "Other apps read aloud with the default engine. Pick Quoquendo under Preferred engine.",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        TextButton(onClick = { bannerDismissed = true }) { Text("Not now") }
                        TextButton(onClick = { openTtsSettings(context) }) {
                            Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = null, Modifier.size(18.dp))
                            Spacer(Modifier.width(8.dp))
                            Text("Open TTS settings")
                        }
                    }
                }
            }
        }
        if (model.installed.isEmpty()) {
            Empty(model, go)
            return@Column
        }
        VoicePicker(model, voice, go)
        OutlinedTextField(
            value = model.text,
            onValueChange = model::updateText,
            label = { Text("Text") },
            modifier = Modifier.fillMaxWidth().heightIn(min = 132.dp),
            textStyle = MaterialTheme.typography.bodyLarge,
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.AutoMirrored.Outlined.VolumeUp, contentDescription = "Volume", tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Slider(
                value = model.volume,
                onValueChange = model::updateVolume,
                modifier = Modifier.weight(1f).padding(horizontal = 12.dp),
                colors = SliderDefaults.colors(inactiveTrackColor = MaterialTheme.colorScheme.surfaceContainerHighest),
            )
            Text("${(model.volume * 100).toInt()}%", style = MaterialTheme.typography.bodySmall, modifier = Modifier.width(36.dp), textAlign = TextAlign.End)
        }
        val canSpeak = voice != null && voice.ready && model.ttsReady
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            if (model.speaking) {
                Button(onClick = model::stop) {
                    Icon(Icons.Filled.Stop, contentDescription = null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Stop")
                }
            } else {
                Button(onClick = model::speak, enabled = canSpeak) {
                    Icon(Icons.Filled.PlayArrow, contentDescription = null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Speak")
                }
            }
            Box {
                var saveMenu by remember { mutableStateOf(false) }
                OutlinedButton(onClick = { saveMenu = true }, enabled = voice != null && voice.ready) {
                    Icon(Icons.Outlined.Save, contentDescription = null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Save")
                }
                DropdownMenu(expanded = saveMenu, onDismissRequest = { saveMenu = false }) {
                    DropdownMenuItem(text = { Text("WAV") }, onClick = { saveMenu = false; model.save(mp3 = false) })
                    DropdownMenuItem(text = { Text("MP3") }, onClick = { saveMenu = false; model.save(mp3 = true) })
                }
            }
        }
        val status = model.status ?: voice?.let { v -> model.cannotSpeak(v)?.let { Status(Status.Kind.Warn, it) } }
        status?.let { StatusLine(it) }
        ListItem(
            colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
            modifier = Modifier.clickable { model.updateShowCurtain(!model.showCurtain) },
            headlineContent = { Text("Show how it's built") },
            supportingContent = { Text(tryCaption) },
            trailingContent = { Switch(checked = model.showCurtain, onCheckedChange = model::updateShowCurtain) },
        )
        if (model.showCurtain) model.analysis?.let { Curtain(model, it) }
        Spacer(Modifier.height(16.dp))
    }
}

@Composable
private fun Empty(model: MainModel, go: (Screen) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(top = 48.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("No voices installed yet", style = MaterialTheme.typography.titleMedium)
        Text(
            "Download one of the ${model.catalog.size} Loquendo 7 voices, or import an installer .exe you already have.",
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Button(onClick = { go(Screen.Voices) }) { Text("Browse the catalog") }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun VoicePicker(model: MainModel, voice: InstalledVoice?, go: (Screen) -> Unit) {
    var open by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = open, onExpandedChange = { open = it }) {
        OutlinedTextField(
            value = voice?.let { model.title(it) } ?: "",
            onValueChange = {},
            readOnly = true,
            label = { Text("Voice") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = open) },
            modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            model.installed.forEach { v ->
                val why = v.unsupported ?: if (v.languageMissing) "needs ${v.motherTongue} language data" else null
                DropdownMenuItem(
                    text = {
                        Column {
                            Text(model.title(v))
                            if (why != null) Text(why, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    },
                    enabled = v.unsupported == null,
                    onClick = { model.selectVoice(v); open = false },
                )
            }
            HorizontalDivider()
            DropdownMenuItem(text = { Text("Get more voices…") }, onClick = { open = false; go(Screen.Voices) })
        }
    }
}

@Composable
private fun StatusLine(s: Status) {
    val (icon, color) = when (s.kind) {
        Status.Kind.Ok -> Icons.Outlined.CheckCircle to Recorded
        Status.Kind.Info -> Icons.Outlined.Info to MaterialTheme.colorScheme.onSurfaceVariant
        Status.Kind.Warn -> Icons.Outlined.WarningAmber to MaterialTheme.colorScheme.primary
        Status.Kind.Error -> Icons.Outlined.ErrorOutline to MaterialTheme.colorScheme.error
        Status.Kind.Busy -> null to MaterialTheme.colorScheme.onSurfaceVariant
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        if (icon != null) {
            Icon(icon, contentDescription = null, Modifier.size(16.dp), tint = color)
            Spacer(Modifier.width(6.dp))
        } else if (!s.text.startsWith("Speaking")) {
            CircularProgressIndicator(Modifier.size(14.dp), strokeWidth = 2.dp)
            Spacer(Modifier.width(8.dp))
        }
        Text(s.text, style = MaterialTheme.typography.bodySmall, color = color)
    }
}

@Composable
private fun Heading(title: String, rest: androidx.compose.ui.text.AnnotatedString.Builder.() -> Unit) {
    val primary = MaterialTheme.colorScheme.primary
    Text(
        buildAnnotatedString {
            withStyle(SpanStyle(color = primary, fontWeight = FontWeight.Medium)) { append(title) }
            append(" · ")
            rest()
        },
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(top = 8.dp),
    )
}

@Composable
private fun Panel(content: @Composable () -> Unit) {
    Column(
        Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainer, RoundedCornerShape(16.dp)).padding(16.dp),
    ) { content() }
}

private val mono = FontFamily.Monospace

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Curtain(model: MainModel, a: Analysis) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val sounds = a.phrases.sumOf { it.phones.size }
    Heading("Sounds") {
        append("$sounds · ")
        withStyle(SpanStyle(color = Recorded)) { append("green") }
        append(" recorded by ${a.voice}, ")
        withStyle(SpanStyle(color = Missing)) { append("red") }
        append(" never recorded")
    }
    Panel {
        val bar = MaterialTheme.colorScheme.outlineVariant
        val primary = MaterialTheme.colorScheme.primary
        FlowRow(verticalArrangement = Arrangement.spacedBy(6.dp), horizontalArrangement = Arrangement.spacedBy(0.dp)) {
            a.phrases.forEach { p ->
                // one item per word, so lines wrap between words
                val words = mutableListOf<MutableList<AnPhone>>()
                p.phones.forEach { ph -> if (ph.wordStart || words.isEmpty()) words += mutableListOf(ph) else words.last() += ph }
                words.forEachIndexed { w, word ->
                    if (w > 0) Text("|", fontFamily = mono, fontSize = 13.sp, color = bar, modifier = Modifier.padding(horizontal = 5.dp))
                    Text(
                        buildAnnotatedString {
                            word.forEachIndexed { i, ph ->
                                if (i > 0) append(" ")
                                withStyle(SpanStyle(color = if (ph.known) Recorded else Missing)) { append(ph.sym) }
                            }
                        },
                        fontFamily = mono,
                        fontSize = 13.sp,
                    )
                }
                when (p.mood) {
                    "I" -> Text("?", fontFamily = mono, fontWeight = FontWeight.Bold, color = primary, modifier = Modifier.padding(start = 4.dp))
                    "E" -> Text("!", fontFamily = mono, fontWeight = FontWeight.Bold, color = primary, modifier = Modifier.padding(start = 4.dp))
                }
                if (p.pauseMs > 0) {
                    Text(
                        "${p.pauseMs} ms",
                        fontFamily = mono,
                        fontSize = 10.sp,
                        color = muted,
                        modifier = Modifier.padding(horizontal = 5.dp)
                            .border(BorderStroke(1.dp, MaterialTheme.colorScheme.outline), RoundedCornerShape(4.dp))
                            .padding(horizontal = 4.dp, vertical = 1.dp),
                    )
                }
            }
        }
    }

    val units = a.pieces.map { it.unit }.toSet().size
    Heading("Pieces") {
        append("${a.pieces.size} from $units sentences · ")
        withStyle(SpanStyle(color = HardCut)) { append("purple") }
        append(" = hard cut")
    }
    var all by remember(a) { mutableStateOf(false) }
    Panel {
        val small = MaterialTheme.typography.bodySmall.copy(fontSize = 12.sp)
        Row(Modifier.padding(bottom = 6.dp)) {
            Text("sounds", style = small, color = muted, modifier = Modifier.weight(1.3f))
            Text("sentence", style = small, color = muted, modifier = Modifier.weight(1f))
            Text("at", style = small, color = muted, modifier = Modifier.weight(0.9f), textAlign = TextAlign.End)
            Text("len", style = small, color = muted, modifier = Modifier.weight(0.5f), textAlign = TextAlign.End)
            Text("pitch", style = small, color = muted, modifier = Modifier.weight(0.6f), textAlign = TextAlign.End)
        }
        val shown = if (all) a.pieces.size else minOf(a.pieces.size, 12)
        a.pieces.take(shown).forEachIndexed { index, p ->
            val playing = model.playingPiece == index
            Row(
                Modifier.fillMaxWidth()
                    .background(if (playing) MaterialTheme.colorScheme.primaryContainer else Color.Transparent, RoundedCornerShape(6.dp))
                    .clickable { model.playPiece(index) }
                    .padding(vertical = 5.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(Modifier.weight(1.3f), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Filled.PlayArrow, contentDescription = "Play", Modifier.size(14.dp).alpha(if (playing) 1f else 0.5f), tint = MaterialTheme.colorScheme.primary)
                    Spacer(Modifier.width(2.dp))
                    Text(p.covers, fontFamily = mono, fontSize = 12.sp, color = if (p.hard) HardCut else Recorded, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                Text("#${p.unit}", fontFamily = mono, fontSize = 12.sp, color = muted, modifier = Modifier.weight(1f))
                Text("%.2f".format(Locale.ROOT, p.atS), fontFamily = mono, fontSize = 12.sp, modifier = Modifier.weight(0.9f), textAlign = TextAlign.End)
                Text("%.0f".format(Locale.ROOT, p.lenMs), fontFamily = mono, fontSize = 12.sp, modifier = Modifier.weight(0.5f), textAlign = TextAlign.End)
                Text("${p.pitch}", fontFamily = mono, fontSize = 12.sp, modifier = Modifier.weight(0.6f), textAlign = TextAlign.End)
            }
            // after a phrase's last piece: what it never recorded, and its pause
            val last = a.pieces.getOrNull(index + 1)?.phrase != p.phrase
            val planned = a.plan.getOrNull(p.phrase)
            if (last && planned != null) {
                if (planned.missing.isNotEmpty()) {
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 3.dp)) {
                        Icon(Icons.Outlined.WarningAmber, contentDescription = null, Modifier.size(14.dp), tint = Missing)
                        Spacer(Modifier.width(4.dp))
                        Text("${planned.missing.joinToString(" ")}: never recorded — nearest sound used", fontSize = 12.sp, color = Missing)
                    }
                }
                if (planned.pauseMs > 0) Text("— pause ${planned.pauseMs} ms", fontSize = 11.sp, color = muted, modifier = Modifier.padding(start = 16.dp, bottom = 2.dp))
            }
        }
        Text(
            "Tap a piece to hear it as it was recorded",
            style = MaterialTheme.typography.bodySmall,
            color = muted,
            modifier = Modifier.padding(top = 8.dp),
        )
        if (a.pieces.size > shown) {
            Text(
                "Show all ${a.pieces.size} pieces",
                color = MaterialTheme.colorScheme.primary,
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(top = 8.dp).clickable { all = true },
            )
        }
    }

    a.audio?.let { audio ->
        Heading("Audio") {
            append("%.2f s · %s Hz · %s samples".format(Locale.ROOT, audio.seconds, grouped(audio.rate.toLong()), grouped(audio.samples)))
        }
        val color = MaterialTheme.colorScheme.primary
        val head = MaterialTheme.colorScheme.onSurface
        val playhead = model.playheadS
        Panel {
            Canvas(Modifier.fillMaxWidth().height(72.dp)) {
                val env = audio.envelope
                if (env.isEmpty()) return@Canvas
                val mid = size.height / 2
                val step = size.width / (env.size - 1).coerceAtLeast(1)
                val path = Path().apply {
                    moveTo(0f, mid)
                    env.forEachIndexed { i, e -> lineTo(i * step, mid - e * mid) }
                    for (i in env.indices.reversed()) lineTo(i * step, mid + env[i] * mid)
                    close()
                }
                drawLine(color.copy(alpha = 0.3f), Offset(0f, mid), Offset(size.width, mid), 1f)
                drawPath(path, color)
                if (playhead != null && audio.seconds > 0 && model.analysis?.voice == model.selectedVoice()?.name) {
                    val x = (playhead / audio.seconds).toFloat().coerceIn(0f, 1f) * size.width
                    drawLine(head, Offset(x, 0f), Offset(x, size.height), 3f)
                }
            }
        }
    }
}
