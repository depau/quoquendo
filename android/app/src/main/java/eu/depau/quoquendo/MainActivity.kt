package eu.depau.quoquendo

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.graphics.Color as AndroidColor
import android.os.Bundle
import android.speech.tts.TextToSpeech
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.PlayCircle
import androidx.compose.material.icons.outlined.RecordVoiceOver
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Snackbar
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp

/** The app's screens; those with a [parent] open from it, outside the navigation bar. */
enum class Screen(val title: String, val icon: ImageVector, val parent: Screen? = null) {
    Voices("Voices", Icons.Outlined.RecordVoiceOver),
    Try("Try it", Icons.Outlined.PlayCircle),
    Settings("Settings", Icons.Outlined.Settings),
    Profiles("Profiles", Icons.Outlined.Tune, Settings),
    Requests("Speech requests", Icons.Outlined.History, Settings),
}

class MainActivity : ComponentActivity() {
    private val model: MainModel by viewModels()
    private val screen = mutableStateOf(Screen.Try)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
        )
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) route(intent)
        setContent {
            QuoquendoTheme {
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
        model.refreshDefaultEngine()
    }

    private fun route(intent: Intent?) {
        screen.value = when {
            intent?.action == TextToSpeech.Engine.ACTION_INSTALL_TTS_DATA -> Screen.Voices
            // the system's engine settings open the EngineSettings alias
            intent?.component?.className?.endsWith(".EngineSettings") == true -> Screen.Settings
            else -> when (intent?.getStringExtra("screen")) {
                "voices" -> Screen.Voices
                "settings" -> Screen.Settings
                else -> screen.value
            }
        }
    }
}

/** Open the system text-to-speech settings. */
fun openTtsSettings(context: Context) {
    try {
        context.startActivity(Intent("com.android.settings.TTS_SETTINGS"))
    } catch (_: ActivityNotFoundException) {
        context.startActivity(Intent(android.provider.Settings.ACTION_SETTINGS))
    }
}

@Composable
fun App(model: MainModel, screen: Screen, go: (Screen) -> Unit) {
    val snackbar = remember { SnackbarHostState() }
    val errorSnack = remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        for (s in model.snacks) {
            errorSnack.value = s.error
            val result = snackbar.showSnackbar(
                s.message,
                actionLabel = if (s.retry != null) "Retry" else null,
                duration = if (s.retry != null || s.error) SnackbarDuration.Long else SnackbarDuration.Short,
            )
            if (result == SnackbarResult.ActionPerformed) s.retry?.invoke()
        }
    }
    val search = remember { mutableStateOf<String?>(null) }
    val requestSearch = remember { mutableStateOf<String?>(null) }
    screen.parent?.let { parent -> BackHandler { go(parent) } }
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surface,
        topBar = {
            when (screen) {
                Screen.Voices -> VoicesTopBar(model, search)
                Screen.Try -> TryTopBar(model)
                Screen.Settings -> SettingsTopBar()
                Screen.Profiles -> ProfilesTopBar(go)
                Screen.Requests -> RequestsTopBar(model, go, requestSearch)
            }
        },
        snackbarHost = {
            SnackbarHost(snackbar) { data ->
                Snackbar(
                    action = data.visuals.actionLabel?.let { label ->
                        { TextButton(onClick = data::performAction) { Text(label, color = MaterialTheme.colorScheme.inversePrimary) } }
                    },
                    modifier = Modifier.padding(12.dp),
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (errorSnack.value) {
                            // the snackbar is the inverse surface: light error red on dark, dark red on light
                            val tint = if (isSystemInDarkTheme()) Color(0xFFBA1A1A) else Color(0xFFFFB4AB)
                            Icon(Icons.Outlined.ErrorOutline, contentDescription = null, tint = tint, modifier = Modifier.size(20.dp))
                            Spacer(Modifier.width(12.dp))
                        }
                        Text(data.visuals.message)
                    }
                }
            }
        },
        bottomBar = {
            NavigationBar(containerColor = MaterialTheme.colorScheme.surfaceContainer) {
                Screen.entries.filter { it.parent == null }.forEach { s ->
                    NavigationBarItem(
                        selected = s == screen || s == screen.parent,
                        onClick = { go(s) },
                        icon = { Icon(s.icon, contentDescription = null) },
                        label = { Text(s.title) },
                        colors = NavigationBarItemDefaults.colors(indicatorColor = MaterialTheme.colorScheme.primaryContainer),
                    )
                }
            }
        },
    ) { pad ->
        val modifier = Modifier.padding(pad)
        when (screen) {
            Screen.Voices -> VoicesScreen(model, search.value, modifier)
            Screen.Try -> TryScreen(model, go, modifier)
            Screen.Settings -> SettingsScreen(model, go, modifier)
            Screen.Profiles -> ProfilesScreen(model, go, modifier)
            Screen.Requests -> RequestsScreen(model, requestSearch.value, modifier)
        }
    }
}
