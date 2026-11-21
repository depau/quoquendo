package eu.depau.quoquendo

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.automirrored.outlined.VolumeUp
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.RestartAlt
import androidx.compose.material.icons.outlined.Save
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.rememberTextMeasurer
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

/** `Roberto — Italian (Male)` */
private fun MainModel.title(v: InstalledVoice) = "${v.name} — ${languageOf(v)} (${v.gender})"

/** `generic` → `Generic` */
private fun capitalized(s: String) = s.replaceFirstChar { it.titlecase(Locale.ROOT) }

/** What is playing at the playhead: the sound, its word and the piece (indices into the analysis). */
private class Moment(val seconds: Double, val sound: Int?, val word: Int?, val piece: Int?)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun TryScreen(model: MainModel, go: (Screen) -> Unit, modifier: Modifier) {
    val context = LocalContext.current
    var bannerDismissed by rememberSaveable { mutableStateOf(false) }
    val voice = model.selectedVoice()
    // an empty text box starts with the voice's own sentence
    LaunchedEffect(voice?.name) {
        if (voice != null && model.text.isBlank()) model.updateText(voice.demoSentence)
    }
    LaunchedEffect(model.installed) { model.scheduleAnalysis() }
    val a = model.analysis?.takeIf { it.voice == voice?.name }
    val moment = model.spoken?.takeIf { it === a }?.let { s ->
        model.playheadS?.let { t ->
            val sound = s.soundAt(t)
            Moment(t, sound, sound?.let { s.sounds[it].word }, s.pieceAt(t))
        }
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
                        Text("Quoquendo isn't your default text-to-speech engine", style = MaterialTheme.typography.titleSmall)
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
            Empty(go)
            return@Column
        }
        VoicePicker(model, voice, go)
        if (model.advanced) model.details?.takeIf { it.voice == voice?.name }?.let { LanguagePicker(model, it) }
        TextBox(model, a?.takeIf { moment != null && it.text == model.text }, moment)
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
        val canRender = voice != null && voice.ready && model.text.isNotBlank()
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                when (model.playback) {
                    Playback.Playing -> Button(onClick = model::pause, contentPadding = ButtonDefaults.ButtonWithIconContentPadding) {
                        Icon(Icons.Filled.Pause, contentDescription = null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Pause")
                    }
                    Playback.Paused -> Button(onClick = model::speak, contentPadding = ButtonDefaults.ButtonWithIconContentPadding) {
                        Icon(Icons.Filled.PlayArrow, contentDescription = null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Resume")
                    }
                    Playback.Stopped -> Button(onClick = model::speak, enabled = canRender, contentPadding = ButtonDefaults.ButtonWithIconContentPadding) {
                        Icon(Icons.Filled.PlayArrow, contentDescription = null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Speak")
                    }
                }
                OutlinedIconButton(onClick = model::stop, enabled = model.playback != Playback.Stopped) {
                    Icon(Icons.Filled.Stop, contentDescription = "Stop")
                }
            }
            SaveControl(model, enabled = canRender && !model.saving)
        }
        val status = model.status ?: voice?.let { v -> model.cannotSpeak(v)?.let { Status(Status.Kind.Warn, it) } }
        status?.let { StatusLine(it) }
        if (a != null) Curtain(model, a, moment)
        if (model.advanced) model.details?.takeIf { it.voice == voice?.name }?.let { Details(it) }
        Spacer(Modifier.height(16.dp))
    }
}

@Composable
private fun Empty(go: (Screen) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(top = 48.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("No voices installed yet", style = MaterialTheme.typography.titleMedium)
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
            leadingIcon = voice?.let { v -> { Flag(model.languageOf(v), Modifier.size(20.dp, 14.dp)) } },
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
                    leadingIcon = { Flag(model.languageOf(v), Modifier.size(24.dp, 17.dp).alpha(if (v.unsupported == null) 1f else 0.38f)) },
                    enabled = v.unsupported == null,
                    onClick = { model.selectVoice(v); open = false },
                )
            }
            HorizontalDivider()
            DropdownMenuItem(text = { Text("Get more voices…") }, onClick = { open = false; go(Screen.Voices) })
        }
    }
}

/** Advanced: the language the text is read by. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LanguagePicker(model: MainModel, d: VoiceDetails) {
    var open by remember { mutableStateOf(false) }
    val current = model.language ?: d.language
    ExposedDropdownMenuBox(expanded = open, onExpandedChange = { open = it }) {
        OutlinedTextField(
            value = capitalized(current),
            onValueChange = {},
            readOnly = true,
            label = { Text("Language") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = open) },
            modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            d.languages.forEach { l ->
                DropdownMenuItem(text = { Text(capitalized(l)) }, onClick = { model.updateLanguage(l); open = false })
            }
        }
    }
}

/** Marks the word being spoken and dims the ones already spoken. */
private class Highlight(val current: AnWord?, val spokenEnd: Int, val dim: Color, val mark: Color) : VisualTransformation {
    override fun filter(text: AnnotatedString): TransformedText {
        val n = text.length
        val out = AnnotatedString.Builder(text).apply {
            if (spokenEnd > 0) addStyle(SpanStyle(color = dim), 0, spokenEnd.coerceAtMost(n))
            current?.let { w -> if (w.start < w.end && w.end <= n) addStyle(SpanStyle(background = mark), w.start, w.end) }
        }.toAnnotatedString()
        return TransformedText(out, OffsetMapping.Identity)
    }
}

@Composable
private fun TextBox(model: MainModel, live: Analysis?, moment: Moment?) {
    val transformation = if (live != null && moment != null) {
        val current = moment.word?.let { live.words.getOrNull(it) }
        val spokenEnd = live.words.filter { it.endS != null && it.endS <= moment.seconds }.maxOfOrNull { it.end } ?: 0
        Highlight(current, minOf(spokenEnd, current?.start ?: spokenEnd), MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f), MaterialTheme.colorScheme.primaryContainer)
    } else {
        VisualTransformation.None
    }
    OutlinedTextField(
        value = model.text,
        onValueChange = model::updateText,
        label = { Text("Text") },
        trailingIcon = {
            IconButton(onClick = model::sampleText) { Icon(Icons.Outlined.RestartAlt, contentDescription = "Sample text") }
        },
        visualTransformation = transformation,
        modifier = Modifier.fillMaxWidth().heightIn(min = 96.dp),
        textStyle = MaterialTheme.typography.bodyLarge,
    )
}

/** The split control: the format first, then Save. */
@Composable
private fun SaveControl(model: MainModel, enabled: Boolean) {
    var menu by remember { mutableStateOf(false) }
    val padding = PaddingValues(horizontal = 12.dp)
    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Box {
            OutlinedButton(onClick = { menu = true }, contentPadding = padding) {
                Text(model.saveFormat.label)
                Spacer(Modifier.width(4.dp))
                Icon(if (menu) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown, contentDescription = "Format", Modifier.size(18.dp))
            }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                model.formats.forEach { f ->
                    val selected = f == model.saveFormat
                    DropdownMenuItem(
                        modifier = if (selected) Modifier.background(MaterialTheme.colorScheme.secondaryContainer) else Modifier,
                        leadingIcon = { Icon(Icons.Filled.Check, contentDescription = null, Modifier.alpha(if (selected) 1f else 0f)) },
                        text = {
                            Column {
                                Text(f.label, style = MaterialTheme.typography.bodyLarge)
                                Text(capitalized(f.hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        },
                        onClick = { model.updateSaveFormat(f); menu = false },
                    )
                }
            }
        }
        OutlinedButton(onClick = model::save, enabled = enabled, contentPadding = padding) {
            Icon(Icons.Outlined.Save, contentDescription = null, Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text("Save")
        }
    }
}

@Composable
private fun StatusLine(s: Status) {
    val (icon, color) = when (s.kind) {
        Status.Kind.Warn -> Icons.Outlined.WarningAmber to MaterialTheme.colorScheme.primary
        Status.Kind.Error -> Icons.Outlined.ErrorOutline to MaterialTheme.colorScheme.error
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, contentDescription = null, Modifier.size(16.dp), tint = color)
        Spacer(Modifier.width(6.dp))
        Text(s.text, style = MaterialTheme.typography.bodySmall, color = color)
    }
}

@Composable
private fun Heading(title: String, rest: AnnotatedString.Builder.() -> Unit) {
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

/** Waveform width per second of speech, when it is longer than the screen is wide. */
private const val DP_PER_SECOND = 120

@Composable
private fun Curtain(model: MainModel, a: Analysis, moment: Moment?) {
    Sounds(a, moment)
    Pieces(model, a, moment)
    a.audio?.let { Audio(a, it, moment) }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Sounds(a: Analysis, moment: Moment?) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val primary = MaterialTheme.colorScheme.primary
    val wordMark = MaterialTheme.colorScheme.primaryContainer
    val onPrimary = MaterialTheme.colorScheme.onPrimary
    val recorded = Recorded
    val missing = Missing
    Heading("Sounds") {
        withStyle(SpanStyle(color = recorded)) { append("green") }
        append(" recorded · ")
        withStyle(SpanStyle(color = missing)) { append("red") }
        append(" never recorded")
    }
    Panel {
        val bar = MaterialTheme.colorScheme.outlineVariant
        val phrases = maxOf(a.plan.size, (a.sounds.maxOfOrNull { it.phrase } ?: -1) + 1)
        FlowRow(verticalArrangement = Arrangement.spacedBy(6.dp), horizontalArrangement = Arrangement.spacedBy(0.dp)) {
            for (p in 0 until phrases) {
                // one item per word, so lines wrap between words
                val words = mutableListOf<MutableList<Int>>()
                a.sounds.forEachIndexed { i, s ->
                    if (s.phrase != p || s.pause) return@forEachIndexed
                    if (s.wordStart || words.isEmpty()) words += mutableListOf(i) else words.last() += i
                }
                words.forEachIndexed { w, word ->
                    if (w > 0) Text("|", fontFamily = mono, fontSize = 13.sp, color = bar, modifier = Modifier.padding(horizontal = 5.dp))
                    Text(
                        buildAnnotatedString {
                            word.forEachIndexed { k, i ->
                                val s = a.sounds[i]
                                val inWord = moment?.word != null && s.word == moment.word
                                if (k > 0) withStyle(SpanStyle(background = if (inWord) wordMark else Color.Unspecified)) { append(" ") }
                                val style = when {
                                    i == moment?.sound -> SpanStyle(color = onPrimary, background = primary)
                                    inWord -> SpanStyle(color = if (s.known) recorded else missing, background = wordMark)
                                    else -> SpanStyle(color = if (s.known) recorded else missing)
                                }
                                withStyle(style) { append(s.sym) }
                            }
                        },
                        fontFamily = mono,
                        fontSize = 13.sp,
                    )
                }
                val planned = a.plan.getOrNull(p)
                when (planned?.mood) {
                    "I" -> Text("?", fontFamily = mono, fontWeight = FontWeight.Bold, color = primary, modifier = Modifier.padding(start = 4.dp))
                    "E" -> Text("!", fontFamily = mono, fontWeight = FontWeight.Bold, color = primary, modifier = Modifier.padding(start = 4.dp))
                }
                if (planned != null && planned.pauseMs > 0) {
                    Text(
                        "${planned.pauseMs} ms",
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
}

@Composable
private fun Pieces(model: MainModel, a: Analysis, moment: Moment?) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val hardCut = HardCut
    Heading("Pieces") {
        withStyle(SpanStyle(color = hardCut)) { append("purple") }
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
        val shown = if (all) a.pieces.size else minOf(a.pieces.size, PIECES_SHOWN)
        a.pieces.take(shown).forEachIndexed { index, p ->
            val playing = model.playingPiece == index || moment?.piece == index
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
                Text("%.2f".format(Locale.ROOT, p.recS), fontFamily = mono, fontSize = 12.sp, modifier = Modifier.weight(0.9f), textAlign = TextAlign.End)
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
        if (a.pieces.size > shown) {
            Text(
                "Show all ${a.pieces.size} pieces",
                color = MaterialTheme.colorScheme.primary,
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(top = 8.dp).clickable { all = true },
            )
        }
    }
}

private const val PIECES_SHOWN = 12

/**
 * The waveform and, on its time axis, the strip of sounds; both scroll
 * sideways when long, following the playhead. Touch and hold shows the sound
 * under the finger.
 */
@Composable
private fun Audio(a: Analysis, audio: AnAudio, moment: Moment?) {
    val primary = MaterialTheme.colorScheme.primary
    Heading("Audio") {
        if (moment != null) {
            withStyle(SpanStyle(color = primary, fontWeight = FontWeight.Medium)) { append("%.2f s".format(Locale.ROOT, moment.seconds)) }
            append(" / ")
        }
        append("%.2f s · %s Hz".format(Locale.ROOT, audio.seconds, grouped(audio.rate.toLong())))
    }
    val muted = primary.copy(alpha = 0.35f)
    val head = MaterialTheme.colorScheme.onSurface
    val axis = MaterialTheme.colorScheme.outline
    val wordMark = MaterialTheme.colorScheme.primaryContainer
    val pauseBlock = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f)
    val recorded = Recorded
    val missing = Missing
    val measurer = rememberTextMeasurer()
    val labelStyle = TextStyle(fontFamily = mono, fontSize = 9.sp, letterSpacing = (-0.5).sp)
    val labels = remember(a, measurer) { a.sounds.map { if (it.pause) null else measurer.measure(it.sym, labelStyle) } }
    val chipStyle = MaterialTheme.typography.labelMedium.copy(color = MaterialTheme.colorScheme.onSurface)
    val chipBack = MaterialTheme.colorScheme.surfaceContainerHighest
    val scrubMark = MaterialTheme.colorScheme.secondaryContainer
    Panel {
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            val density = LocalDensity.current
            val viewport = constraints.maxWidth.toFloat()
            val width = maxOf(maxWidth, (audio.seconds * DP_PER_SECOND).dp)
            val widthPx = with(density) { width.toPx() }
            val scroll = rememberScrollState()
            var scrub by remember(a) { mutableStateOf<Float?>(null) }
            // keep the playhead in view while speaking
            val x = moment?.let { (it.seconds / audio.seconds).toFloat().coerceIn(0f, 1f) * widthPx }
            LaunchedEffect(x) {
                if (x != null && (x < scroll.value || x > scroll.value + viewport * 0.8f)) scroll.scrollTo((x - viewport * 0.2f).toInt().coerceAtLeast(0))
            }
            Box(Modifier.horizontalScroll(scroll)) {
                Canvas(
                    Modifier.width(width).height(112.dp).pointerInput(a) {
                        // after a long press, so a plain swipe still scrolls
                        detectDragGesturesAfterLongPress(
                            onDragStart = { scrub = it.x },
                            onDrag = { change, _ ->
                                change.consume()
                                scrub = change.position.x.coerceIn(0f, size.width.toFloat())
                            },
                            onDragEnd = { scrub = null },
                            onDragCancel = { scrub = null },
                        )
                    },
                ) {
                    val waveH = 72.dp.toPx()
                    val perS = size.width / audio.seconds.toFloat().coerceAtLeast(0.001f)
                    val env = audio.envelope
                    val mid = waveH / 2
                    if (env.isNotEmpty()) {
                        val step = audio.step.toFloat() / audio.rate * perS
                        val path = Path().apply {
                            moveTo(0f, mid)
                            env.forEachIndexed { i, e -> lineTo((i + 0.5f) * step, mid - e * mid) }
                            lineTo(env.size * step, mid)
                            for (i in env.indices.reversed()) lineTo((i + 0.5f) * step, mid + env[i] * mid)
                            close()
                        }
                        drawLine(primary.copy(alpha = 0.3f), Offset(0f, mid), Offset(size.width, mid), 1f)
                        if (x == null) {
                            drawPath(path, primary)
                        } else {
                            clipRect(right = x) { drawPath(path, primary) }
                            clipRect(left = x) { drawPath(path, muted) }
                        }
                    }
                    // the strip: a tick at every sound boundary (longer at words), labels centred in their span
                    val top = waveH + 6.dp.toPx()
                    val labelTop = top + 9.dp.toPx()
                    val labelH = 14.dp.toPx()
                    val tick = 1.dp.toPx()
                    var lastEnd = Float.NEGATIVE_INFINITY
                    a.sounds.forEachIndexed { i, s ->
                        val x0 = (s.atS * perS).toFloat()
                        val x1 = (s.endS * perS).toFloat()
                        if (s.pause) {
                            drawRoundRect(pauseBlock, Offset(x0 + tick, labelTop), Size((x1 - x0 - 2 * tick).coerceAtLeast(0f), labelH), CornerRadius(2.dp.toPx()))
                            return@forEachIndexed
                        }
                        drawLine(axis, Offset(x0, top), Offset(x0, top + (if (s.wordStart) 8 else 4).dp.toPx()), tick)
                        if (moment?.word != null && s.word == moment.word) drawRect(wordMark, Offset(x0, labelTop), Size(x1 - x0, labelH))
                        // the sound playing shows in the chip on the playhead instead
                        if (i == moment?.sound) return@forEachIndexed
                        val label = labels[i] ?: return@forEachIndexed
                        val lx = (x0 + x1) / 2 - label.size.width / 2
                        // a label that would collide is left out
                        if (lx < lastEnd) return@forEachIndexed
                        drawText(label, color = if (s.known) recorded else missing, topLeft = Offset(lx, labelTop + (labelH - label.size.height) / 2))
                        lastEnd = lx + label.size.width
                    }

                    /** An outlined chip of [text], its left edge from [place] given its width, its top from [top] given its height. */
                    fun chip(text: AnnotatedString, place: (Float) -> Float, top: (Float) -> Float) {
                        val laid = measurer.measure(text, chipStyle)
                        val pad = 6.dp.toPx()
                        val box = Size(laid.size.width + 2 * pad, laid.size.height + pad)
                        val at = Offset(place(box.width), top(box.height))
                        drawRoundRect(chipBack, at, box, CornerRadius(6.dp.toPx()))
                        drawRoundRect(axis, at, box, CornerRadius(6.dp.toPx()), style = Stroke(1.dp.toPx()))
                        drawText(laid, topLeft = Offset(at.x + pad, at.y + pad / 2))
                    }
                    fun AnnotatedString.Builder.sym(s: AnSound) = withStyle(SpanStyle(color = if (s.known) recorded else missing, fontFamily = mono)) { append(s.sym) }

                    if (x != null) {
                        drawLine(head, Offset(x, 4.dp.toPx()), Offset(x, labelTop + labelH), 2.dp.toPx())
                        drawCircle(head, 4.dp.toPx(), Offset(x, 4.dp.toPx()))
                    }
                    // touch and hold: the sound under the finger, its label shown even where it didn't fit
                    val sx = scrub
                    val t = sx?.let { it / perS }
                    val si = t?.let { at -> a.sounds.indexOfLast { it.atS <= at }.takeIf { it >= 0 && at < a.sounds[it].endS } }
                    val s = si?.let { a.sounds[it] }
                    if (sx != null) {
                        val label = si?.let { labels[it] }
                        if (s != null && label != null) {
                            val x0 = (s.atS * perS).toFloat()
                            val w = maxOf((s.endS * perS).toFloat() - x0, label.size.width + 4.dp.toPx())
                            val left = x0 + ((s.endS * perS).toFloat() - x0) / 2 - w / 2
                            drawRect(scrubMark, Offset(left, labelTop), Size(w, labelH))
                            drawText(label, color = if (s.known) recorded else missing, topLeft = Offset(left + (w - label.size.width) / 2, labelTop + (labelH - label.size.height) / 2))
                        }
                        drawLine(head.copy(alpha = 0.45f), Offset(sx, 4.dp.toPx()), Offset(sx, labelTop + labelH), 2.dp.toPx())
                        drawCircle(head.copy(alpha = 0.45f), 4.dp.toPx(), Offset(sx, 4.dp.toPx()))
                    }
                    // the sound playing, on the playhead where it crosses the strip
                    val playing = moment?.sound?.let { a.sounds[it] }?.takeIf { !it.pause }
                    if (x != null && playing != null) {
                        chip(buildAnnotatedString { sym(playing) }, { w -> (x - w / 2).coerceIn(0f, size.width - w) }, { h -> labelTop + labelH / 2 - h / 2 })
                    }
                    // the scrub's chip above the waveform, beside its line on whichever side stays in view
                    if (sx != null && t != null) {
                        chip(
                            buildAnnotatedString {
                                if (s != null && !s.pause) {
                                    sym(s)
                                    append("  ")
                                }
                                append("%.2f s".format(Locale.ROOT, t))
                            },
                            { w -> if (sx + 8.dp.toPx() + w <= scroll.value + viewport) sx + 8.dp.toPx() else sx - 8.dp.toPx() - w },
                            { 10.dp.toPx() },
                        )
                    }
                }
            }
        }
    }
}

/** Advanced: what the voice is made of, collapsible. */
@Composable
private fun Details(d: VoiceDetails) {
    var open by rememberSaveable { mutableStateOf(true) }
    Row(Modifier.fillMaxWidth().clickable { open = !open }.padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("Voice details", color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
        Icon(
            if (open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = if (open) "Collapse" else "Expand",
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
    if (open) {
        Panel {
            d.rows.forEach { (label, value) ->
                Row(Modifier.padding(vertical = 3.dp)) {
                    Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(0.4f))
                    Text(value, fontFamily = mono, fontSize = 12.sp, modifier = Modifier.weight(0.6f))
                }
            }
        }
    }
}
