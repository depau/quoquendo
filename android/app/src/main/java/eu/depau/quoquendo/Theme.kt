package eu.depau.quoquendo

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import java.util.Locale

/** The approved light tonal scheme, seeded from the amber. */
private val Scheme = lightColorScheme(
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

/** Phones the voice recorded / never recorded, and pieces joined at a hard cut. */
val Recorded = Color(0xFF2E7D32)
val Missing = Color(0xFFBA1A1A)
val HardCut = Color(0xFF6750A4)

@Composable
fun QuoquendoTheme(content: @Composable () -> Unit) = MaterialTheme(colorScheme = Scheme, content = content)

/** Decimal sizes as the catalog shows them: `52.8 MB`, `1.9 GB`. */
fun size(bytes: Long): String =
    if (bytes >= 1_000_000_000) "%.1f GB".format(Locale.ROOT, bytes / 1e9)
    else "%.1f MB".format(Locale.ROOT, bytes / 1e6)

/** `326 238`: thousands grouped with a space. */
fun grouped(n: Long): String = "%,d".format(Locale.ROOT, n).replace(',', ' ')
