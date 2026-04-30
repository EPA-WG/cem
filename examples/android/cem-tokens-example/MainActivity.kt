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
import androidx.compose.ui.unit.dp
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
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Button(
            onClick = {},
            modifier = Modifier.heightIn(min = 44.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = Color.fromHex(CEMTokens.cemColorBlueL),
                contentColor = Color.fromHex(CEMTokens.cemColorBlueXd),
            ),
        ) {
            Text("Primary action")
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = Color.fromHex(CEMTokens.cemColorCyanXl)),
        ) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Comfort surface", color = Color.fromHex(CEMTokens.cemColorCyanXd))
                Text("Generated CEM tokens drive color, radius, and spacing.", color = Color.fromHex(CEMTokens.cemColorCyanXd))
            }
        }
    }
}

fun Color.Companion.fromHex(hex: String): Color {
    val value = android.graphics.Color.parseColor(hex)
    return Color(value)
}
