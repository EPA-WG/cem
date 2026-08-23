package org.epawg.cem.example

import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.epawg.cem.tokens.CEMTokens

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            CEMTokenExample()
        }
    }
}

@Composable
fun CEMTokenExample() {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.fromHex(CEMTokens.cemColorCyanXl))
            .padding(CEMTokens.cemInsetSurface.cemDp()),
        verticalArrangement = Arrangement.spacedBy(CEMTokens.cemGapBlock.cemDp()),
    ) {
        Button(
            onClick = {},
            modifier = Modifier.heightIn(min = CEMTokens.cemCouplingZoneMin.cemDp()),
            shape = RoundedCornerShape(CEMTokens.cemBendControl.cemDp()),
            colors = ButtonDefaults.buttonColors(
                containerColor = Color.fromHex(CEMTokens.cemColorBlueL),
                contentColor = Color.fromHex(CEMTokens.cemColorBlueXd),
            ),
        ) {
            Text(
                "Primary action",
                fontSize = CEMTokens.cemTypographySizeM.cemSp(),
                fontWeight = FontWeight(CEMTokens.cemThicknessBold.toInt()),
            )
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(CEMTokens.cemBendSurface.cemDp()),
            colors = CardDefaults.cardColors(containerColor = Color.fromHex(CEMTokens.cemColorCyanXl)),
        ) {
            Column(
                Modifier.padding(CEMTokens.cemInsetContainer.cemDp()),
                verticalArrangement = Arrangement.spacedBy(CEMTokens.cemGapRelated.cemDp()),
            ) {
                Text(
                    "Comfort surface",
                    color = Color.fromHex(CEMTokens.cemColorCyanXd),
                    fontSize = CEMTokens.cemTypographySizeM.cemSp(),
                )
                Text(
                    "Generated CEM tokens drive color, radius, and spacing.",
                    color = Color.fromHex(CEMTokens.cemColorCyanXd),
                    fontSize = CEMTokens.cemTypographySizeS.cemSp(),
                )
            }
        }
    }
}

fun Color.Companion.fromHex(hex: String): Color {
    val value = android.graphics.Color.parseColor(hex)
    return Color(value)
}

fun String.cemDp(): Dp {
    val normalized = trim()
    return when {
        normalized.endsWith("rem") -> (normalized.removeSuffix("rem").toFloat() * 16).dp
        normalized.endsWith("px") -> normalized.removeSuffix("px").toFloat().dp
        else -> normalized.toFloat().dp
    }
}

fun String.cemSp(): TextUnit {
    val normalized = trim()
    return when {
        normalized.endsWith("rem") -> (normalized.removeSuffix("rem").toFloat() * 16).sp
        normalized.endsWith("px") -> normalized.removeSuffix("px").toFloat().sp
        else -> normalized.toFloat().sp
    }
}
