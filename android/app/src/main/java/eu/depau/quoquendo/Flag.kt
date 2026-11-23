package eu.depau.quoquendo

import android.graphics.BitmapFactory
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

private val shapes = HashMap<String, JSONArray>()
private val pictures = HashMap<String, ImageBitmap?>()

private val flagShape = RoundedCornerShape(2.dp)

/** Roberto runs the trains: his flag is the Trenitalia mark, not Italy's. */
fun flagLanguage(voice: String, language: String) = if (voice.equals("Roberto", ignoreCase = true)) "Trenitalia" else language

/**
 * A language's flag (`US English`, `EnglishGb`), always rectangular, drawn from
 * the engine's shapes (crates/quoquendo/src/flag.rs); nothing for a language
 * without one. A hairline keeps white stripes apart from the background.
 */
@Composable
fun Flag(language: String, modifier: Modifier = Modifier) {
    val hairline = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.15f)
    val picture = pictures.getOrPut(language) {
        Native.flagImage(language)?.let { BitmapFactory.decodeByteArray(it, 0, it.size)?.asImageBitmap() }
    }
    if (picture != null) {
        Image(
            bitmap = picture,
            contentDescription = null,
            contentScale = ContentScale.FillBounds,
            modifier = modifier.clip(flagShape).border(1.dp, hairline, flagShape),
        )
        return
    }
    val flag = shapes.getOrPut(language) { JSONArray(Native.flag(language)) }
    if (flag.length() == 0) return
    Canvas(modifier.clip(flagShape).border(1.dp, hairline, flagShape)) {
        val sx = size.width / 30f
        val sy = size.height / 21f
        for (i in 0 until flag.length()) {
            val s = flag.getJSONArray(i)
            val color = Color(0xFF000000 or s.getLong(1))
            fun n(k: Int) = s.getDouble(k + 2).toFloat()
            when (s.getString(0)) {
                "rect" -> drawRect(color, Offset(n(0) * sx, n(1) * sy), Size(n(2) * sx, n(3) * sy))
                "circle" -> drawCircle(color, n(2) * sx, Offset(n(0) * sx, n(1) * sy))
                "poly" -> drawPath(Path().apply {
                    moveTo(n(0) * sx, n(1) * sy)
                    for (k in 2 until s.length() - 2 step 2) lineTo(n(k) * sx, n(k + 1) * sy)
                    close()
                }, color)
                "star" -> drawPath(Path().apply {
                    for (k in 0 until 10) {
                        val a = PI * (k / 5.0 - 0.5)
                        val r = if (k % 2 == 0) n(2) else n(3)
                        val x = (n(0) + r * cos(a)).toFloat() * sx
                        val y = (n(1) + r * sin(a)).toFloat() * sy
                        if (k == 0) moveTo(x, y) else lineTo(x, y)
                    }
                    close()
                }, color)
            }
        }
    }
}
