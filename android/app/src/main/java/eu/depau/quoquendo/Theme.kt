package eu.depau.quoquendo

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import java.util.Locale

/** The approved light tonal scheme, seeded from the amber. */
private val Light = lightColorScheme(
    primary = Color(0xFF755B00),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFFFDF9C),
    onPrimaryContainer = Color(0xFF241A00),
    secondaryContainer = Color(0xFFF5E0BB),
    onSecondaryContainer = Color(0xFF241A04),
    background = Color(0xFFFFF8F1),
    onBackground = Color(0xFF1F1B14),
    surface = Color(0xFFFFF8F1),
    onSurface = Color(0xFF1F1B14),
    surfaceVariant = Color(0xFFEFE9DF),
    inverseSurface = Color(0xFF353027),
    inverseOnSurface = Color(0xFFF8EFE3),
    inversePrimary = Color(0xFFE9C349),
    onSurfaceVariant = Color(0xFF4E4639),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFFAF3EA),
    surfaceContainer = Color(0xFFF5EFE6),
    surfaceContainerHigh = Color(0xFFEFE9DF),
    surfaceContainerHighest = Color(0xFFE9E3D9),
    outline = Color(0xFF7F7667),
    outlineVariant = Color(0xFFD2C5B0),
    error = Color(0xFFBA1A1A),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
)

/** Its dark counterpart: the same amber palette at the dark tones. */
private val Dark = darkColorScheme(
    primary = Color(0xFFE9C349),
    onPrimary = Color(0xFF3D2E00),
    primaryContainer = Color(0xFF584400),
    onPrimaryContainer = Color(0xFFFFDF9C),
    secondaryContainer = Color(0xFF53462A),
    onSecondaryContainer = Color(0xFFF5E0BB),
    background = Color(0xFF17130B),
    onBackground = Color(0xFFEBE1D4),
    surface = Color(0xFF17130B),
    onSurface = Color(0xFFEBE1D4),
    surfaceVariant = Color(0xFF4E4639),
    inverseSurface = Color(0xFFEBE1D4),
    inverseOnSurface = Color(0xFF353027),
    inversePrimary = Color(0xFF755B00),
    onSurfaceVariant = Color(0xFFD2C5B0),
    surfaceContainerLowest = Color(0xFF110E07),
    surfaceContainerLow = Color(0xFF1F1B14),
    surfaceContainer = Color(0xFF231F17),
    surfaceContainerHigh = Color(0xFF2E2921),
    surfaceContainerHighest = Color(0xFF393429),
    outline = Color(0xFF9B8F7F),
    outlineVariant = Color(0xFF4E4639),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
)

/** Phones the voice recorded / never recorded, and pieces joined at a hard cut. */
val Recorded: Color @Composable get() = if (isSystemInDarkTheme()) Color(0xFF81C784) else Color(0xFF2E7D32)
val Missing: Color @Composable get() = MaterialTheme.colorScheme.error
val HardCut: Color @Composable get() = if (isSystemInDarkTheme()) Color(0xFFCFBCFF) else Color(0xFF6750A4)

@Composable
fun QuoquendoTheme(content: @Composable () -> Unit) =
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) Dark else Light, content = content)

/** Decimal sizes as the catalog shows them: `52.8 MB`, `1.9 GB`. */
fun size(bytes: Long): String =
    if (bytes >= 1_000_000_000) "%.1f GB".format(Locale.ROOT, bytes / 1e9)
    else "%.1f MB".format(Locale.ROOT, bytes / 1e6)

/** `326 238`: thousands grouped with a space. */
fun grouped(n: Long): String = "%,d".format(Locale.ROOT, n).replace(',', ' ')
